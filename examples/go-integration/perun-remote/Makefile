all: protobuf

install-protoc:
	which protoc >/dev/null || sudo apt-get install protobuf-compiler
	which protoc-gen-go >/dev/null || sudo `which go` install google.golang.org/protobuf/cmd/protoc-gen-go@latest

setup-repo:
	git submodule init && git submodule update
	mkdir -p proto

protobuf: install-protoc setup-repo
	protoc --proto_path=../../../src/wire --proto_path=../../../go-perun/wire/protobuf --go_out=proto --go_opt=paths=source_relative --go_opt=Mperun-remote.proto=remote/proto ../../../src/wire/perun-remote.proto
