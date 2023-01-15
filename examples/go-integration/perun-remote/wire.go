package remote

import (
	"errors"
	"fmt"

	"go-integration/perun-remote/proto"

	"github.com/ethereum/go-ethereum/accounts/abi"

	perun_eth_wallet "github.com/perun-network/perun-eth-backend/wallet"
	"perun.network/go-perun/channel"
	"perun.network/go-perun/wallet"

	perunProto "perun.network/go-perun/wire/protobuf"
)

func verifySigs(sigs []wallet.Sig, state *channel.State, params channel.Params) bool {
	if len(sigs) != len(params.Parts) {
		return false
	}

	for i, sig := range sigs {
		ok, _ := channel.Verify(params.Parts[i], state, sig)
		if !ok {
			return false
		}
	}
	return true
}
func toIdx(i uint32) (channel.Index, error) {
	if i >= 1<<16 {
		return 0, errors.New("invalid index")
	}
	return channel.Index(i), nil
}

type WatchRequestMsg struct {
	Participant channel.Index
	State       channel.SignedState
	AuthSigner  wallet.Account
}

func ParseWatchRequestMsg(p *proto.WatchRequestMsg) (*WatchRequestMsg, error) {
	signed, err := perunProto.ToSignedState(p.State)
	if err != nil {
		return nil, err
	}
	idx, err := toIdx(p.Participant)
	if err != nil {
		return nil, err
	}

	if int(idx) > len(signed.Params.Parts) {
		return nil, errors.New("Invalid participant index")
	}

	signer := NewPreSignedAccount(signed.Params.Parts[int(idx)])

	var (
		abiUint256, _ = abi.NewType("uint256", "", nil)
		abiAddress, _ = abi.NewType("address", "", nil)
		abiBytes32, _ = abi.NewType("bytes32", "", nil)
	)

	for i, auth := range p.WithdrawalAuths {
		args := abi.Arguments{
			{Type: abiBytes32},
			{Type: abiAddress},
			{Type: abiAddress},
			{Type: abiUint256},
		}
		recv := wallet.NewAddress()
		if err := recv.UnmarshalBinary(auth.Receiver); err != nil {
			return nil, fmt.Errorf("decoding receiver address: %w", err)
		}
		enc, err := args.Pack(
			signed.State.ID,
			perun_eth_wallet.AsEthAddr(signer.Address()),
			perun_eth_wallet.AsEthAddr(recv),
			signed.State.Allocation.Balances[i][idx])
		if err != nil {
			return nil, fmt.Errorf(
				"ABI encoding withdrawal auths %d: %w", i, err)
		}
		signer.AddSig(string(enc), auth.Sig)
	}

	return &WatchRequestMsg{
		Participant: idx,
		State:       signed,
		AuthSigner:  signer}, nil
}

func (r WatchRequestMsg) VerifyIntegrity() bool {
	if r.State.State.ID != r.State.Params.ID() {
		return false
	}

	return verifySigs(r.State.Sigs, r.State.State, *r.State.Params)
}

type ForceCloseRequestMsg struct {
	ChannelId channel.ID
	Latest    *WatchRequestMsg
}

func ParseForceCloseRequestMsg(p *proto.ForceCloseRequestMsg) (*ForceCloseRequestMsg, error) {
	var id channel.ID
	copy(id[:], p.ChannelId)

	var latest *WatchRequestMsg
	if p.Latest != nil {
		var err error
		if latest, err = ParseWatchRequestMsg(p.Latest); err != nil {
			return nil, err
		}
	}
	return &ForceCloseRequestMsg{ChannelId: id, Latest: latest}, nil
}

type FundingRequestMsg struct {
	Participant      channel.Index
	Params           channel.Params
	InitialState     channel.State
	FundingAgreement channel.Balances
}

func ParseFundingRequestMsg(p *proto.FundingRequestMsg) (_ *FundingRequestMsg, err error) {
	var req FundingRequestMsg
	if req.Participant, err = toIdx(p.Participant); err != nil {
		return
	}

	params, err := perunProto.ToParams(p.Params)
	if err != nil {
		return nil, err
	}
	req.Params = *params

	initState, err := perunProto.ToState(p.InitialState)
	if err != nil {
		return nil, err
	}
	req.InitialState = *initState

	req.FundingAgreement = perunProto.ToBalances(p.FundingAgreement)

	return &req, nil
}
