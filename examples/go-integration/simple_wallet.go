// SPDX-License-Identifier: Apache-2.0

package main

import (
	"crypto/ecdsa"
	"math/big"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
)

func NewSimpleWallet() *SimpleWallet {
	return &SimpleWallet{
		accounts: make([]accounts.Account, 0),
		keys:     make(map[common.Address]*ecdsa.PrivateKey, 0),
	}
}

type SimpleWallet struct {
	accounts []accounts.Account
	keys     map[common.Address]*ecdsa.PrivateKey
}

var _ accounts.Wallet = (*SimpleWallet)(nil)

func (w *SimpleWallet) GenerateNewAccount() accounts.Account {
	sk, _ := crypto.GenerateKey()
	addr := crypto.PubkeyToAddress(sk.PublicKey)
	account := accounts.Account{Address: addr}

	w.accounts = append(w.accounts, account)
	w.keys[addr] = sk
	return account
}

// Accounts implements accounts.Wallet
func (w *SimpleWallet) Accounts() []accounts.Account {
	cpy := make([]accounts.Account, len(w.accounts))
	copy(cpy, w.accounts)
	return w.accounts
}

// Close implements accounts.Wallet
func (w *SimpleWallet) Close() error {
	panic("unimplemented")
}

// Contains implements accounts.Wallet
func (w *SimpleWallet) Contains(account accounts.Account) bool {
	for i := 0; i < len(w.accounts); i++ {
		if w.accounts[i].Address == account.Address {
			return true
		}
	}
	return false
}

// Derive implements accounts.Wallet
func (*SimpleWallet) Derive(path accounts.DerivationPath, pin bool) (accounts.Account, error) {
	panic("unimplemented")
}

// Open implements accounts.Wallet
func (*SimpleWallet) Open(passphrase string) error {
	panic("unimplemented")
}

// SelfDerive implements accounts.Wallet
func (*SimpleWallet) SelfDerive(bases []accounts.DerivationPath, chain ethereum.ChainStateReader) {
	panic("unimplemented")
}

// SignData implements accounts.Wallet
func (w *SimpleWallet) SignData(account accounts.Account, mimeType string, data []byte) ([]byte, error) {
	hash := crypto.Keccak256(data)
	return crypto.Sign(hash, w.keys[account.Address])
}

// SignDataWithPassphrase implements accounts.Wallet
func (*SimpleWallet) SignDataWithPassphrase(account accounts.Account, passphrase string, mimeType string, data []byte) ([]byte, error) {
	panic("unimplemented")
}

// SignText implements accounts.Wallet
func (w *SimpleWallet) SignText(account accounts.Account, text []byte) ([]byte, error) {
	hash := accounts.TextHash(text)
	return crypto.Sign(hash, w.keys[account.Address])
}

// SignTextWithPassphrase implements accounts.Wallet
func (*SimpleWallet) SignTextWithPassphrase(account accounts.Account, passphrase string, hash []byte) ([]byte, error) {
	panic("unimplemented")
}

// SignTx implements accounts.Wallet
func (w *SimpleWallet) SignTx(account accounts.Account, tx *types.Transaction, chainID *big.Int) (*types.Transaction, error) {
	signer := types.NewLondonSigner(chainID)
	return types.SignTx(tx, signer, w.keys[account.Address])
}

// SignTxWithPassphrase implements accounts.Wallet
func (*SimpleWallet) SignTxWithPassphrase(account accounts.Account, passphrase string, tx *types.Transaction, chainID *big.Int) (*types.Transaction, error) {
	panic("unimplemented")
}

// Status implements accounts.Wallet
func (*SimpleWallet) Status() (string, error) {
	panic("unimplemented")
}

// URL implements accounts.Wallet
func (*SimpleWallet) URL() accounts.URL {
	panic("unimplemented")
}
