package remote

import (
	"encoding/binary"
	"fmt"
	"io"
	"net"

	protobuf "google.golang.org/protobuf/proto"

	log "github.com/sirupsen/logrus"

	"polycry.pt/poly-go/sync"

	"perun.network/go-perun/channel"

	"go-integration/perun-remote/proto"
)

type Server struct {
	sync.Closer

	server net.Listener

	watcher *WatcherService
	funder  *FunderService
}

func NewServer(
	watcher *WatcherService,
	funder *FunderService,
	port uint16,
) (*Server, error) {
	server, err := net.Listen("tcp", fmt.Sprintf(":%d", port))
	if err != nil {
		return nil, fmt.Errorf("listener: %w", err)
	}

	s := &Server{
		server: server,

		watcher: watcher,
		funder:  funder,
	}

	s.OnCloseAlways(func() { server.Close() })

	return s, nil
}

func (s *Server) Serve() {
	for {
		conn, err := s.server.Accept()
		if err != nil {
			return
		}

		go s.handleConn(conn)
	}
}

func (s *Server) handleConn(conn io.ReadWriteCloser) {
	defer conn.Close()
	s.OnCloseAlways(func() { conn.Close() })

	var m sync.Mutex

	for {
		msg, err := recvMsg(conn)
		if err != nil {
			log.Errorf("decoding message failed: %v", err)
			return
		}

		go func() {
			switch msg := msg.GetMsg().(type) {
			case *proto.Message_WatchRequest:
				log.Warn("Server: Got watch request")
				req, err := ParseWatchRequestMsg(msg.WatchRequest)
				if err != nil {
					log.Errorf("Invalid watch message: %v", err)
					return
				}
				if err = s.watcher.Watch(*req); err != nil {
					log.Errorf("Watching channel failed: %v", err)
				}
				sendMsg(&m, conn, &proto.Message{Msg: &proto.Message_WatchResponse{
					WatchResponse: &proto.WatchResponseMsg{
						ChannelId: req.State.State.ID[:],
						Version:   req.State.State.Version,
						Success:   err == nil}}})
			case *proto.Message_WatchUpdate:
				log.Warn("Server: Got update notification")
				req, err := ParseWatchUpdateMsg(msg.WatchUpdate)
				if err != nil {
					log.Errorf("Invalid update message: %v", err)
					return
				}
				if err = s.watcher.Update(*req); err != nil {
					log.Errorf("Invalid update received: %v", err)
				}
				sendMsg(&m, conn, &proto.Message{Msg: &proto.Message_WatchResponse{
					WatchResponse: &proto.WatchResponseMsg{
						ChannelId: req.InitialState.ID[:],
						Version:   req.InitialState.Version,
						Success:   err == nil}}})
			case *proto.Message_ForceCloseRequest:
				log.Warn("Server: Got dispute request")
				req, err := ParseForceCloseRequestMsg(msg.ForceCloseRequest)
				if err != nil {
					log.Errorf("Invalid force-close message: %v", err)
					return
				}
				if err := s.watcher.StartDispute(*req); err != nil {
					log.Errorf("Disputing failed: %v", err)
				}
				sendMsg(&m, conn, &proto.Message{Msg: &proto.Message_ForceCloseResponse{
					ForceCloseResponse: &proto.ForceCloseResponseMsg{
						ChannelId: req.ChannelId[:],
						Success:   err == nil}}})
			case *proto.Message_FundingRequest:
				log.Warn("Server: Got Funding request")
				req, err := ParseFundingRequestMsg(msg.FundingRequest)
				if err != nil {
					log.Errorf("Invalid update message: %v", err)
					return
				}
				if err := s.funder.Fund(s.Ctx(), channel.FundingReq{
					Params:    &req.Params,
					State:     &req.InitialState,
					Idx:       req.Participant,
					Agreement: req.FundingAgreement,
				}); err != nil {
					log.Errorf("Funding failed: %v", err)
				}
				sendMsg(&m, conn, &proto.Message{Msg: &proto.Message_FundingResponse{
					FundingResponse: &proto.FundingResponseMsg{
						ChannelId: req.InitialState.ID[:],
						Success:   err == nil}}})
			}
		}()
	}
}

func recvMsg(conn io.Reader) (*proto.Message, error) {
	var size uint16
	if err := binary.Read(conn, binary.BigEndian, &size); err != nil {
		return nil, fmt.Errorf("reading size of data from wire: %w", err)
	}
	data := make([]byte, size)
	if _, err := io.ReadFull(conn, data); err != nil {
		return nil, fmt.Errorf("reading data from wire: %w", err)
	}
	var msg proto.Message
	if err := protobuf.Unmarshal(data, &msg); err != nil {
		return nil, fmt.Errorf("unmarshalling message: %w", err)
	}
	return &msg, nil
}

func sendMsg(m *sync.Mutex, conn io.Writer, msg *proto.Message) error {
	m.Lock()
	defer m.Unlock()
	data, err := protobuf.Marshal(msg)
	if err != nil {
		return fmt.Errorf("marshalling message: %w", err)
	}
	if err := binary.Write(conn, binary.BigEndian, uint16(len(data))); err != nil {
		return fmt.Errorf("writing length to wire: %w", err)
	}
	if _, err = conn.Write(data); err != nil {
		return fmt.Errorf("writing data to wire: %w", err)
	}
	return nil
}
