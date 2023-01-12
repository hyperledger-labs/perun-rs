package remote

import (
	"errors"
	"go-integration/perun-remote/proto"

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
	return &WatchRequestMsg{Participant: idx, State: signed}, nil
}

func (r WatchRequestMsg) VerifyIntegrity() bool {
	if r.State.State.ID != r.State.Params.ID() {
		return false
	}

	return verifySigs(r.State.Sigs, r.State.State, *r.State.Params)
}

type WatchUpdateMsg struct {
	InitialState channel.State
	Sigs         []wallet.Sig
}

func ParseWatchUpdateMsg(p *proto.WatchUpdateMsg) (*WatchUpdateMsg, error) {
	state, err := perunProto.ToState(p.InitialState)
	if err != nil {
		return nil, err
	}
	return &WatchUpdateMsg{InitialState: *state, Sigs: p.Sigs}, nil
}

func (u *WatchUpdateMsg) VerifyIntegrity(params channel.Params, version uint64) error {
	if u.InitialState.ID != params.ID() {
		return errors.New("invalid channel ID")
	}

	if u.InitialState.Version < version {
		return errors.New("outdated version")
	}

	if !verifySigs(u.Sigs, &u.InitialState, params) {
		return errors.New("invalid signatures")
	}
	return nil
}

type ForceCloseRequestMsg struct {
	ChannelId channel.ID
	Latest    *WatchUpdateMsg
}

func ParseForceCloseRequestMsg(p *proto.ForceCloseRequestMsg) (*ForceCloseRequestMsg, error) {
	var id channel.ID
	copy(id[:], p.ChannelId)

	var latest *WatchUpdateMsg
	if p.Latest != nil {
		var err error
		if latest, err = ParseWatchUpdateMsg(p.Latest); err != nil {
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
