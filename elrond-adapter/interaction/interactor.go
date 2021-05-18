package interaction

import (
	"sync"

	"github.com/ElrondNetwork/elrond-adapter/config"
	"github.com/ElrondNetwork/elrond-sdk/erdgo"
	"github.com/ElrondNetwork/elrond-sdk/erdgo/blockchain"
	"github.com/ElrondNetwork/elrond-sdk/erdgo/data"
)

type BlockchainInteractor struct {
	proxyUrl   string
	chainID    string
	gasLimit   uint64
	gasPrice   uint64
	privateKey []byte
	publicKey  string
	account    *data.Account
	txMut      sync.Mutex
}

func NewBlockchainInteractor(chainInfo config.BlockchainInformation) (*BlockchainInteractor, error) {
	sk, pk, err := GetKeyPairFromPem(chainInfo.PemPath)
	if err != nil {
		return nil, err
	}

	addressHandler, err := data.NewAddressFromBech32String(pk)
	if err != nil {
		return nil, err
	}

	proxy := blockchain.NewElrondProxy(chainInfo.ProxyUrl)
	account, err := proxy.GetAccount(addressHandler)
	if err != nil {
		return nil, err
	}

	return &BlockchainInteractor{
		proxyUrl:   chainInfo.ProxyUrl,
		chainID:    chainInfo.ChainID,
		gasLimit:   chainInfo.GasLimit,
		gasPrice:   chainInfo.GasPrice,
		privateKey: sk,
		publicKey:  pk,
		account:    account,
	}, nil
}

func (bi *BlockchainInteractor) SendTx(tx *data.Transaction) (string, error) {
	bi.txMut.Lock()
	defer bi.txMut.Unlock()

	proxy := blockchain.NewElrondProxy(bi.proxyUrl)
	txHash, err := proxy.SendTransaction(tx)
	if err != nil {
		return "", err
	}

	bi.account.Nonce++
	return txHash, nil
}

func (bi *BlockchainInteractor) CreateSignedTx(
	value string,
	inputData []byte,
	receiver string,
) (*data.Transaction, error) {
	tx := &data.Transaction{
		Value:    value,
		RcvAddr:  receiver,
		Data:     inputData,
		Nonce:    bi.account.Nonce,
		SndAddr:  bi.account.Address,
		GasPrice: bi.gasPrice,
		GasLimit: bi.gasLimit,
		ChainID:  bi.chainID,
		Version:  1,
		Options:  0,
	}

	err := erdgo.SignTransaction(tx, bi.privateKey)
	if err != nil {
		return nil, err
	}

	return tx, nil
}

func (bi *BlockchainInteractor) GetOwnAccount() data.Account {
	return *bi.account
}

func (bi *BlockchainInteractor) GetKeyPair() ([]byte, string) {
	return bi.privateKey, bi.publicKey
}
