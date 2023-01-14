package remote

import (
	"errors"

	"github.com/ethereum/go-ethereum/crypto"
	"perun.network/go-perun/wallet"
)

// PreSignedAccount exposes are set of precomputed signatures as a wallet.Account.
type PreSignedAccount struct {
	address    wallet.Address
	signatures map[string]wallet.Sig
}

var _ wallet.Account = (*PreSignedAccount)(nil)

func NewPreSignedAccount(addr wallet.Address) *PreSignedAccount {
	return &PreSignedAccount{
		address:    addr,
		signatures: make(map[string]wallet.Sig)}
}

func (p PreSignedAccount) Address() wallet.Address { return p.address }

func (p *PreSignedAccount) AddSig(msgHash string, sig wallet.Sig) {
	p.signatures[msgHash] = sig
}

func (p *PreSignedAccount) SignData(data []byte) ([]byte, error) {
	hash := crypto.Keccak256(data)

	if sig, ok := p.signatures[string(hash[:])]; ok {
		return sig, nil
	}

	return nil, errors.New("PreSignedAccount: unanticipated request.")
}
