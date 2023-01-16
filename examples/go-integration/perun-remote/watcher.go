package remote

import (
	"context"
	"errors"
	"fmt"
	"sync"

	log "github.com/sirupsen/logrus"

	"perun.network/go-perun/channel"
	"perun.network/go-perun/wallet"
	"perun.network/go-perun/watcher"
)

type watchEntry struct {
	Params channel.Params
	Idx    channel.Index
	watcher.StatesPub
	watcher.AdjudicatorSub
	participantAcc wallet.Account // use PreSignedAccount for secure noncustodial signing
	latest         channel.Transaction
}

// WatcherService serves a single client, watching and disputing multiple ledger channels.
type WatcherService struct {
	mutex sync.Mutex
	watch watcher.Watcher

	watching map[channel.ID]*watchEntry
	adj      channel.Adjudicator
}

func NewWatcherService(
	watch watcher.Watcher,
	adj channel.Adjudicator,
) *WatcherService {
	return &WatcherService{
		watch:    watch,
		watching: make(map[channel.ID]*watchEntry),
		adj:      adj}
}

func (service *WatcherService) Watch(r WatchRequestMsg) error {
	if !r.VerifyIntegrity() {
		return errors.New("invalid request")
	}

	id := r.State.State.ID

	latestTx := channel.Transaction{
		State: r.State.State,
		Sigs:  r.State.Sigs,
	}

	// register channel if not tracked, otherwise, update.
	entry, err := func() (_ *watchEntry, _ error) {
		service.mutex.Lock()
		defer service.mutex.Unlock()

		entry, ok := service.watching[id]
		if ok {
			if r.State.State.Version < entry.latest.State.Version {
				return nil, errors.New("registered outdated version")
			}

			entry.latest.State = r.State.State
			entry.latest.Sigs = r.State.Sigs
			entry.participantAcc = r.AuthSigner
			return entry, nil
		} else {
			// This should ideally happen in another thread / outside of the master mutex lock, but for now it's alright.
			pub, sub, err := service.watch.StartWatchingLedgerChannel(
				context.Background(), r.State)
			if err != nil {
				return nil, err
			}
			entry = &watchEntry{
				Params:         *r.State.Params,
				Idx:            r.Participant,
				StatesPub:      pub,
				AdjudicatorSub: sub,
				participantAcc: r.AuthSigner,
				latest:         latestTx}
			service.watching[id] = entry

			go service.watchAndWithdraw(entry)
			return entry, nil
		}
	}()
	if err != nil {
		return err
	}

	err = entry.Publish(context.Background(), latestTx)
	if err != nil {
		log.Errorf("Watcher: publishing channel: %v", err)
	}

	if r.State.State.IsFinal {
		log.Warn("Final state reached, withdrawing...")
		err := service.adj.Register(context.Background(), channel.AdjudicatorReq{
			Params: &entry.Params,
			Acc:    entry.participantAcc,
			Tx:     entry.latest,
			Idx:    entry.Idx,
		}, nil)

		if err != nil {
			return fmt.Errorf("Failed to withdraw: %w", err)
		}
		log.Warn("Successfully withdrawn!")
	}

	return nil
}

func (service *WatcherService) watchAndWithdraw(e *watchEntry) error {
	defer service.watch.StopWatching(context.Background(), e.Params.ID())
	defer log.Warnln("watchAndWithdraw returns.")
	for evt := range e.EventStream() {
		if _, ok := evt.(*channel.ConcludedEvent); ok {
			break
		} else {
			log.Warnf("Awaiting timout on adjudicator event: %T", evt)
			log.Warnf("Wait: %v", evt.Timeout().Wait(context.Background()))
			log.Warnf("Timeout %T elapsed", evt)
			break
		}
	}

	req := func() channel.AdjudicatorReq {
		service.mutex.Lock()
		defer service.mutex.Unlock()
		return channel.AdjudicatorReq{
			Params: &e.Params,
			Acc:    e.participantAcc,
			Tx:     e.latest,
			Idx:    e.Idx}
	}()

	log.Warnln("Channel concluded on-chain! withdrawing...")
	err := service.adj.Withdraw(context.Background(), req, nil)

	if err != nil {
		log.Errorf("Failed to withdraw: %v", err)
	}
	log.Warn("Successfully withdrawn!")
	return nil
}

func (service *WatcherService) StartDispute(u ForceCloseRequestMsg) error {
	service.mutex.Lock()
	entry, ok := service.watching[u.ChannelId]
	service.mutex.Unlock()
	if !ok {
		return errors.New("disputing unknown channel")
	}

	if u.Latest != nil {
		err := service.Watch(*u.Latest)
		if err != nil {
			panic(err)
		}
		// Do not register twice.
		if u.Latest.State.State.IsFinal {
			return nil
		}
	}

	req := func() channel.AdjudicatorReq {
		service.mutex.Lock()
		defer service.mutex.Unlock()
		return channel.AdjudicatorReq{
			Params: &entry.Params,
			Acc:    entry.participantAcc,
			Tx:     entry.latest,
			Idx:    entry.Idx}
	}()

	log.Warnln("Registering state for dispute...")
	err := service.adj.Register(context.Background(), req, nil)

	if err != nil {
		return fmt.Errorf("Failed to dispute: %w", err)
	}
	log.Warn("Successfully registered!")
	return nil
}
