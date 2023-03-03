package control

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"math/big"
	"net"
	"strconv"
	"strings"
	"sync"

	"github.com/ethereum/go-ethereum/common"
	ethchannel "github.com/perun-network/perun-eth-backend/channel"
	ethwallet "github.com/perun-network/perun-eth-backend/wallet"
	"perun.network/go-perun/channel"
	"perun.network/go-perun/client"
	"perun.network/go-perun/wire"
	"perun.network/go-perun/wire/net/simple"
)

type ControlService struct {
	mu          sync.Mutex
	channelsIds []channel.ID
	client      *client.Client
	eth_holder  common.Address
	participant common.Address
}

func NewControlService(cl *client.Client, eth_holder common.Address, participant common.Address) ControlService {
	return ControlService{
		mu:          sync.Mutex{},
		channelsIds: make([]channel.ID, 0),
		client:      cl,
		eth_holder:  eth_holder,
		participant: participant,
	}
}

func (s *ControlService) Run() error {
	l, err := net.Listen("tcp", ":2222")
	if err != nil {
		panic(err)
	}
	for {
		conn, err := l.Accept()
		if err != nil {
			return err
		}
		go s.connHandler(conn)
	}
}

func (s *ControlService) connHandler(conn net.Conn) {
	r := bufio.NewScanner(conn)
	w := bufio.NewWriter(conn)
	writeString := func(str string) {
		_, err := w.WriteString(str)
		if err != nil {
			panic(err)
		}
		err = w.Flush()
		if err != nil {
			panic(err)
		}
	}
	writeString("Participant control service\nWrite h for help\n> ")
	for r.Scan() {
		cmd := r.Text()
		if cmd == "q" || cmd == "quit" {
			break
		}
		err := s.processCmd(cmd, w)
		if err != nil {
			writeString(err.Error())
		}
		writeString("> ")
	}
}

func (s *ControlService) processCmd(cmd string, w *bufio.Writer) error {
	writeString := func(str string) {
		_, err := w.WriteString(str)
		if err != nil {
			panic(err)
		}
		err = w.Flush()
		if err != nil {
			panic(err)
		}
	}

	s.mu.Lock()
	defer s.mu.Unlock()

	c := strings.Split(cmd, " ")
	cmd = c[0]
	args := c[1:]

	switch cmd {
	case "h", "help":
		writeString("" +
			"  h, help            Print this message\n" +
			"  q, quit            Exit the control service (the go-side is still running afterwards)\n" +
			"  p, propose         Propose a channel\n" +
			"  u, update          Update the current channel\n" +
			"  c, close [<index>] Close the channel\n" +
			"  f, force-close     Force close the channel\n" +
			"  s, status          Short status report on the channel\n",
		)
	case "p", "propose":
		err := s.propose_channel()
		if err != nil {
			writeString(err.Error())
		}
	case "u", "update":
		writeString("Not yet implemented\n")
	case "c", "close":
		switch len(args) {
		case 0:
			return s.close_last_channel()
		case 1:
			index, err := strconv.Atoi(args[0])
			if err != nil {
				return err
			}
			return s.close_channel(index)
		}
	case "f", "force-close":
		writeString("Not yet implemented\n")
	case "s", "status":
		s.printStatus(w)
	default:
		writeString("Unknown command\n")
	}
	return nil
}

func (s *ControlService) RegisterChannel(ch *client.Channel) {
	s.mu.Lock()
	defer s.mu.Unlock()

	s.registerChannel(ch)
}

func (s *ControlService) registerChannel(ch *client.Channel) {
	s.channelsIds = append(s.channelsIds, ch.ID())
	ch.OnUpdate(func(from, to *channel.State) {
		if to.IsFinal {
			go func() {
				err := ch.Settle(context.Background(), false)
				if err != nil {
					panic(err)
				}
			}()
		}
	})
	go func() {
		err := ch.Watch(adjudicatorEventHandler{channel: ch})
		if err != nil {
			panic(err)
		}
	}()
}

func (s *ControlService) propose_channel() error {
	peers := []wire.Address{simple.NewAddress("Alice"), simple.NewAddress("Bob")}
	initBals := &channel.Allocation{
		Assets: []channel.Asset{
			&ethchannel.Asset{
				ChainID: ethchannel.ChainID{
					Int: big.NewInt(1337),
				},
				AssetHolder: ethwallet.Address(s.eth_holder),
			},
		},
		Balances: [][]*big.Int{
			{
				big.NewInt(100_000),
				big.NewInt(100_000),
			},
		},
		Locked: []channel.SubAlloc{},
	}
	addr := ethwallet.Address(s.participant)
	proposal, err := client.NewLedgerChannelProposal(16, &addr, initBals, peers)
	if err != nil {
		return err
	}
	ch, err := s.client.ProposeChannel(context.Background(), proposal)
	if err != nil {
		return err
	}
	s.registerChannel(ch)
	return nil
}

type adjudicatorEventHandler struct {
	channel *client.Channel
}

func (h adjudicatorEventHandler) HandleAdjudicatorEvent(channel.AdjudicatorEvent) {
	err := h.channel.Settle(context.Background(), false)
	if err != nil {
		panic(err)
	}
}

func (s *ControlService) close_last_channel() error {
	return s.close_channel(len(s.channelsIds) - 1)
}

func (s *ControlService) close_channel(index int) error {
	if index >= len(s.channelsIds) {
		return fmt.Errorf("Index out of bounds")
	}
	ch, err := s.client.Channel(s.channelsIds[index])
	if err != nil {
		return err
	}
	err = ch.Close()
	if err != nil {
		return err
	}
	return nil
}

func (s *ControlService) printStatus(w io.Writer) {
	fmt_str := "%-5v %-9v %-8v %-12s %-7v %v %s\n"
	fmt.Fprintf(w, fmt_str, "open", "type", "part_idx", "phase", "version", "state", "")

	for _, id := range s.channelsIds {
		ch, err := s.client.Channel(id)
		if err != nil {
			fmt.Fprintf(w, "<%v>", err)
			continue
		}

		channelType := "<unknown>"
		if ch.IsLedgerChannel() {
			channelType = "Ledger"
		} else if ch.IsSubChannel() {
			channelType = "Subledger"
		} else if ch.IsVirtualChannel() {
			channelType = "Virtual"
		}

		phase := ch.Phase()
		state := ch.State()
		isFinal := ""
		if state.IsFinal {
			isFinal = "<final>"
		}

		balances := state.Allocation.Balances

		fmt.Fprintf(w, fmt_str, !ch.IsClosed(), channelType, ch.Idx(), phase.String(), state.Version, balances, isFinal)
	}
}
