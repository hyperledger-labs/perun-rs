package remote

import (
	"context"
	"errors"
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
	latest channel.Transaction
}

// WatcherService serves a single client, watching and disputing multiple ledger channels.
type WatcherService struct {
	mutex sync.Mutex
	watch watcher.Watcher

	watching map[channel.ID]*watchEntry
	adj      channel.Adjudicator
	acc      wallet.Account
}

func NewWatcherService(
	watch watcher.Watcher,
	adj channel.Adjudicator,
	acc wallet.Account,
) *WatcherService {
	return &WatcherService{
		watch:    watch,
		watching: make(map[channel.ID]*watchEntry),
		adj:      adj,
		acc:      acc}
}

func (service *WatcherService) Watch(r WatchRequestMsg) error {
	if !r.VerifyIntegrity() {
		return errors.New("invalid request")
	}

	id := r.State.State.ID

	service.mutex.Lock()
	defer service.mutex.Unlock()

	if _, ok := service.watching[id]; ok {
		return errors.New("already watched")
	}

	pub, sub, err := service.watch.StartWatchingLedgerChannel(
		context.Background(), r.State)
	if err != nil {
		return err
	}

	latest := channel.Transaction{
		State: r.State.State,
		Sigs:  r.State.Sigs,
	}
	service.watching[id] = &watchEntry{
		Params:         *r.State.Params,
		Idx:            r.Participant,
		StatesPub:      pub,
		AdjudicatorSub: sub,
		latest:         latest}

	err = pub.Publish(context.Background(), latest)
	if err != nil {
		log.Errorf("Watcher: publishing channel: %v", err)
	}

	return nil
}

func (service *WatcherService) watchAndWithdraw(e *watchEntry) error {
	defer service.watch.StopWatching(context.Background(), e.Params.ID())
	for evt := range e.EventStream() {
		if _, ok := evt.(*channel.ConcludedEvent); ok {
			break
		} else {
			evt.Timeout().Wait(context.Background())
		}
	}

	return service.adj.Withdraw(
		context.Background(),
		channel.AdjudicatorReq{
			Params: &e.Params,
			Acc:    service.acc,
			Tx:     e.latest,
			Idx:    e.Idx},
		nil)
}

func (service *WatcherService) Update(u WatchUpdateMsg) error {
	id := u.InitialState.ID

	service.mutex.Lock()
	defer service.mutex.Unlock()

	entry, ok := service.watching[id]
	if !ok {
		log.Errorf("Watcher: updating channel: unregistered channel %v", id)
		return errors.New("unregistered channel")
	}

	if err := u.VerifyIntegrity(entry.Params, entry.latest.State.Version); err != nil {
		return err
	}

	entry.latest.State = &u.InitialState
	entry.latest.Sigs = u.Sigs

	if entry.latest.State.IsFinal {
		log.Warn("Final state reached, both sides can now withdraw (not implemented in this integration test)")
		return service.adj.Register(context.Background(), channel.AdjudicatorReq{
			Params: &entry.Params,
			Acc:    service.acc,
			Tx:     entry.latest,
			Idx:    entry.Idx,
		}, nil)
	}

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
		err := service.Update(*u.Latest)
		if err != nil {
			panic(err)
		}
	}

	return service.adj.Register(context.Background(), channel.AdjudicatorReq{
		Params: &entry.Params,
		Acc:    service.acc,
		Tx:     entry.latest,
		Idx:    entry.Idx,
	}, nil)
}
