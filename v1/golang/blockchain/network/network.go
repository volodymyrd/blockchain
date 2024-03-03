package network

import (
	"encoding/json"
	"fmt"
	"net"
	"strings"
	"time"
)

type Package struct {
	Option int
	Data   string
}

const (
	EndBytes = "\000\005\007\001\001\007\005\000"
	WaitTime = 5
	DMaxSize = 2 << 20 // 2 * 2^20 = 2MiB
	BuffSize = 4 << 10 // 4 * 2^10 = 4 Kib
)

type Listener net.Listener
type Conn net.Conn

// Listen address ip:port
func Listen(address string, handle func(Conn, *Package)) Listener {
	splitted := strings.Split(address, ":")
	if len(splitted) != 2 {
		return nil
	}
	listener, err := net.Listen("tcp", "0.0.0.0:"+splitted[1])
	if err != nil {
		return nil
	}
	go serve(listener, handle)
	return Listener(listener)
}

func Handle(option int, conn Conn, pack *Package, handle func(p *Package) string) bool {
	if option != pack.Option {
		return false
	}
	conn.Write([]byte(SerializePackage(&Package{Option: option, Data: handle(pack)}) + EndBytes))
	return true
}
func serve(listener net.Listener, handle func(Conn, *Package)) {
	defer listener.Close()
	for {
		conn, err := listener.Accept()
		if err != nil {
			break
		}
		go handleConn(conn, handle)
	}
}

func handleConn(conn net.Conn, handle func(Conn, *Package)) {
	defer conn.Close()
	pack := readPackage(conn)
	if pack == nil {
		return
	}
	handle(conn, pack)
}
func Send(address string, pack *Package) *Package {
	conn, err := net.Dial("tcp", address)
	if err != nil {
		fmt.Println("Error open connect")
		return nil
	}
	//fmt.Println("Connect is open")
	defer conn.Close()
	conn.Write([]byte(SerializePackage(pack) + EndBytes))
	var (
		res = new(Package)
		ch  = make(chan bool)
	)
	go func() {
		res = readPackage(conn)
		ch <- true
	}()
	select {
	case <-ch:
	case <-time.After(WaitTime * time.Second):
	}
	return res
}

func SerializePackage(pack *Package) string {
	jsonData, err := json.MarshalIndent(*pack, "", "\t")
	if err != nil {
		return ""
	}
	return string(jsonData)
}

func DeserializePackage(data string) *Package {
	//fmt.Printf("DeserializePackage %s ...\n", data)
	var pack Package
	err := json.Unmarshal([]byte(data), &pack)
	if err != nil {
		return nil
	}
	return &pack
}

func readPackage(conn net.Conn) *Package {
	var (
		size   = uint64(0)
		buffer = make([]byte, BuffSize)
		data   string
	)
	for {
		length, err := conn.Read(buffer)
		//fmt.Printf("Read %d bytes\n", length)
		if err != nil {
			return nil
		}
		size += uint64(length)
		if size > DMaxSize {
			return nil
		}
		data = string(buffer[:length])
		//fmt.Printf("Got data %s bytes\n", data)
		if strings.Contains(data, EndBytes) {
			data = strings.Split(data, EndBytes)[0]
			break
		}
	}
	return DeserializePackage(data)
}
