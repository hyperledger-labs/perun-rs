package main

import (
	"context"
	"math/big"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/ethereum/go-ethereum/accounts"
	"github.com/ethereum/go-ethereum/accounts/abi/bind/backends"
	"github.com/ethereum/go-ethereum/core"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/params"
	ethchannel "github.com/perun-network/perun-eth-backend/channel"
	phd "github.com/perun-network/perun-eth-backend/wallet/hd"
	"perun.network/go-perun/channel"
	"perun.network/go-perun/client"
	"perun.network/go-perun/watcher/local"
	"perun.network/go-perun/wire"
	wirenet "perun.network/go-perun/wire/net"
	"perun.network/go-perun/wire/net/simple"
	"perun.network/go-perun/wire/protobuf"
)

func ToWei(value int64, denomination string) *big.Int {
	// if denomination == "ether" or denomination == "eth":
	var m int64
	switch strings.ToLower(denomination) {
	case "ether":
		m = params.Ether
	case "eth":
		m = params.Ether
	case "gwei":
		m = params.GWei
	case "wei":
		m = params.Wei
	default:
		panic("Unknown denomination")
	}
	return new(big.Int).Mul(big.NewInt(value), big.NewInt(m))
}

func main() {
	w := NewSimpleWallet()
	account := w.GenerateNewAccount()
	deployer_account := w.GenerateNewAccount()

	// Setup the simulated backend + wrappers around them
	sk, _ := crypto.GenerateKey()
	addr := crypto.PubkeyToAddress(sk.PublicKey)
	sb := backends.NewSimulatedBackend(
		core.GenesisAlloc{
			addr:                     {Balance: ToWei(1_000_000, "ether")},
			deployer_account.Address: {Balance: ToWei(1_000_000, "ether")},
		},
		30_000_000,
	)
	chain_id := sb.Blockchain().Config().ChainID
	cb := ethchannel.NewContractBackend(
		sb,
		ethchannel.MakeChainID(chain_id),
		NewChainIdAwareTransactor(w, chain_id),
		1,
	)

	go func() {
		for {
			sb.Commit()
			time.Sleep(10 * time.Millisecond)
		}
	}()

	// Deploy contracts
	adjAddr, err := ethchannel.DeployAdjudicator(context.Background(), cb, deployer_account)
	if err != nil {
		panic(err)
	}
	_, err = ethchannel.DeployETHAssetholder(context.Background(), cb, adjAddr, deployer_account)
	if err != nil {
		panic(err)
	}

	// Setup dependency injection objects
	funder := ethchannel.NewFunder(cb)
	adjudicator := ethchannel.NewAdjudicator(
		cb,
		adjAddr,
		account.Address,
		account,
	)
	perunID := wire.NewAddress()
	bus := wirenet.NewBus(
		simple.NewAccount(simple.NewAddress("Bob")),
		simple.NewTCPDialer(time.Minute),
		protobuf.Serializer(),
	)
	wallet, err := phd.NewWallet(w, accounts.DefaultBaseDerivationPath.String(), 0)
	if err != nil {
		panic(err)
	}
	watcher, err := local.NewWatcher(adjudicator)
	if err != nil {
		panic(err)
	}

	c, err := client.New(perunID, bus, funder, adjudicator, wallet, watcher)
	if err != nil {
		panic(err)
	}

	var proposalHandler client.ProposalHandler = ProposalHandler{}
	var updateHandler client.UpdateHandler = UpdateHandler{}

	listener, err := simple.NewTCPListener("127.0.0.1:1337")
	if err != nil {
		panic(err)
	}

	go c.Handle(proposalHandler, updateHandler)
	go bus.Listen(listener)

	// Wait for Ctrl+C
	println("Press Ctrl+C to stop")
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGINT, syscall.SIGTERM)
	<-stop

	c.Close()
	bus.Close()
	println("Done")
}

type ProposalHandler struct{}

// HandleProposal implements client.ProposalHandler
func (ProposalHandler) HandleProposal(proposal client.ChannelProposal, res *client.ProposalResponder) {
	println("HandleProposal(): ", proposal, res)
}

type UpdateHandler struct{}

// HandleUpdate implements client.UpdateHandler
func (UpdateHandler) HandleUpdate(state *channel.State, update client.ChannelUpdate, res *client.UpdateResponder) {
	println("HandleUpdate(): ", state, res)
}
