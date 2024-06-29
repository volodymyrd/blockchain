package blockchain

import (
	"crypto/rsa"
	"database/sql"
	"os"
	"time"
)

type BlockChain struct {
	DB    *sql.DB
	index uint64
}

type Transaction struct {
	RandBytes []byte
	PrevBlock []byte
	Sender    string
	Receiver  string
	Value     uint64
	ToStorage uint64
	CurrHash  []byte
	Signature []byte
}

type Block struct {
	CurrHash     []byte
	PrevHash     []byte
	Nonce        uint64
	Difficulty   uint8
	Miner        string
	Signature    []byte
	Timestamp    time.Time
	Transactions []Transaction
	Mapping      map[string]uint64
}

type User struct {
	PrivateKey *rsa.PrivateKey
}

const (
	CreateTable = `
	create table block_chain (
	    id 
	)
`
)

func NewChain(filename, receiver string) error {
	file, err := os.Create(filename)
	if err != nil {
		return err
	}
	file.Close()
	db, err := sql.Open("sqlite3", filename)
	if err != nil {
		return err
	}
	defer db.Close()
	_, err = db.Exec(CreateTable)
	chain := BlockChain{DB: db}
	genesis := &Block{
		CurrHash:  []byte(GenesisBlock),
		Mapping:   make(map[string]uint64),
		Miner:     receiver,
		Timestamp: time.Now(),
	}
	genesis.Mapping[StorageChain] = StorageValue
	genesis.Mapping[receiver] = GenesisReward
	chain.AddBlock(genesis)
	return nil
}
