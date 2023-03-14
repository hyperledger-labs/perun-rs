// SPDX-License-Identifier: Apache-2.0

package main

import (
	"errors"
	"math/big"

	"github.com/ethereum/go-ethereum/accounts"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
)

// ChainIdAwareTransactor can be used to make TransactOpts for accounts stored in a HD wallet.
type ChainIdAwareTransactor struct {
	Wallet  accounts.Wallet
	ChainId *big.Int
}

// NewTransactor returns a TransactOpts for the given account. It errors if the account is
// not contained in the wallet used for initializing transactor backend.
func (t *ChainIdAwareTransactor) NewTransactor(account accounts.Account) (*bind.TransactOpts, error) {
	if !t.Wallet.Contains(account) {
		return nil, errors.New("account not found in wallet")
	}
	return &bind.TransactOpts{
		From: account.Address,
		Signer: func(address common.Address, tx *types.Transaction) (*types.Transaction, error) {
			if address != account.Address {
				return nil, errors.New("not authorized to sign this account")
			}

			return t.Wallet.SignTx(account, tx, t.ChainId)
		},
	}, nil
}

// NewTransactor returns a backend that can make TransactOpts for accounts
// contained in the given ethereum wallet.
func NewChainIdAwareTransactor(w accounts.Wallet, chainId *big.Int) *ChainIdAwareTransactor {
	return &ChainIdAwareTransactor{Wallet: w, ChainId: chainId}
}
