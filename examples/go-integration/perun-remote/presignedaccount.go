package remote

import (
	"errors"

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

func (p *PreSignedAccount) AddSig(message []byte, sig wallet.Sig) {
	p.signatures[string(message)] = sig
}

func (p *PreSignedAccount) SignData(message []byte) ([]byte, error) {
	if sig, ok := p.signatures[string(message)]; ok {
		return sig, nil
	}

	return nil, errors.New("PreSignedAccount: unanticipated request.")
}
