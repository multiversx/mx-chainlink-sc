package interaction

import (
	"github.com/ElrondNetwork/elrond-sdk-erdgo"
)

func GetKeyPairFromPem(filepath string) ([]byte, string, error) {
	sk, err := erdgo.LoadPrivateKeyFromPemFile(filepath)
	if err != nil {
		return []byte{}, "", err
	}
	pk, err := erdgo.GetAddressFromPrivateKey(sk)
	if err != nil {
		return []byte{}, "", err
	}
	return sk, pk, nil
}
