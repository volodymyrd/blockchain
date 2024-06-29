package main

import (
	"blockchain/network"
	"fmt"
	"strings"
	"time"
)

const (
	ToUpper = iota + 1
	ToLower
)
const (
	Address = ":8080"
)

func main() {
	go network.Listen(Address, handleServer)

	time.Sleep(500 * time.Millisecond)

	res := network.Send(Address, &network.Package{Option: ToUpper, Data: "Hello, World!"})
	fmt.Println(res.Data)

	res = network.Send(Address, &network.Package{Option: ToLower, Data: "Hello, World!"})
	fmt.Println(res.Data)
}

func handleServer(conn network.Conn, pack *network.Package) {
	network.Handle(ToUpper, conn, pack, handleToUpper)
	network.Handle(ToLower, conn, pack, handleToLower)
}

func handleToLower(p *network.Package) string {
	return strings.ToLower(p.Data)
}

func handleToUpper(p *network.Package) string {
	return strings.ToUpper(p.Data)
}
