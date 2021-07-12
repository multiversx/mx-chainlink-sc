package interaction

import (
	"net/http"
	"sync"

	"github.com/ElrondNetwork/elrond-adapter/config"
	logger "github.com/ElrondNetwork/elrond-go-logger"
	"github.com/ElrondNetwork/elrond-sdk-erdgo"
	"github.com/ElrondNetwork/elrond-sdk-erdgo/blockchain"
	"github.com/ElrondNetwork/elrond-sdk-erdgo/data"
)

var log = logger.GetOrCreate("interaction")

type BlockchainInteractor struct {
	proxyUrl   string
	chainID    string
	gasLimit   uint64
	gasPrice   uint64
	nonce      uint64
	privateKey []byte
	publicKey  string
	proxy      blockchain.ProxyHandler
	txMut      sync.Mutex
}

func NewBlockchainInteractor(chainInfo config.BlockchainInformation) (*BlockchainInteractor, error) {
	sk, pk, err := GetKeyPairFromPem(chainInfo.PemPath)
	if err != nil {
		return nil, err
	}

	proxy := blockchain.NewElrondProxy(chainInfo.ProxyUrl, &http.Client{})

	addressHandler, err := data.NewAddressFromBech32String(pk)
	if err != nil {
		return nil, err
	}

	account, err := proxy.GetAccount(addressHandler)
	if err != nil {
		return nil, err
	}

	return &BlockchainInteractor{
		proxyUrl:   chainInfo.ProxyUrl,
		chainID:    chainInfo.ChainID,
		gasLimit:   chainInfo.GasLimit,
		gasPrice:   chainInfo.GasPrice,
		nonce:      account.Nonce,
		privateKey: sk,
		publicKey:  pk,
		proxy:      proxy,
	}, nil
}

func (bi *BlockchainInteractor) SendTx(tx *data.Transaction) (string, error) {
	txHash, err := bi.proxy.SendTransaction(tx)
	if err != nil {
		log.Debug("failed sending transaction", "err", err.Error())
		return "", err
	}

	log.Info("current local nonce", "nonce", tx.Nonce)
	return txHash, nil
}

func (bi *BlockchainInteractor) CreateSignedTx(
	value string,
	inputData []byte,
	receiver string,
) (*data.Transaction, error) {
	bi.txMut.Lock()
	defer bi.txMut.Unlock()

	account, err := bi.getAccount()
	if err != nil {
		return nil, err
	}
	if account.Nonce > bi.nonce {
		log.Debug("got higher nonce from proxy",
			"nonce", account.Nonce,
			"current nonce", bi.nonce,
			"replacing", true,
		)
		bi.nonce = account.Nonce
	}

	tx := &data.Transaction{
		Value:    value,
		RcvAddr:  receiver,
		Data:     inputData,
		SndAddr:  account.Address,
		Nonce:    bi.nonce,
		GasPrice: bi.gasPrice,
		GasLimit: bi.gasLimit,
		ChainID:  bi.chainID,
		Version:  1,
		Options:  0,
	}

	err = erdgo.SignTransaction(tx, bi.privateKey)
	if err != nil {
		log.Debug("failed signing transaction", "err", err.Error())
		return nil, err
	}

	bi.nonce++
	return tx, nil
}

func (bi *BlockchainInteractor) getAccount() (*data.Account, error) {
	addressHandler, err := data.NewAddressFromBech32String(bi.publicKey)
	if err != nil {
		return nil, err
	}

	account, err := bi.proxy.GetAccount(addressHandler)
	if err != nil {
		return nil, err
	}
	return account, nil
}

func (bi *BlockchainInteractor) GetKeyPair() ([]byte, string) {
	return bi.privateKey, bi.publicKey
}
