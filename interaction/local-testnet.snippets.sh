USERS="../my-testnet/testnet/wallets/users"
PROXY=http://localhost:7950
METACHAIN_ADDRESS="erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u"

wallet_of() {
    local NAME=$1
    echo "${USERS}/${NAME}.pem"
}

address_of() {
    local WALLET=$(wallet_of $1)
    (set -x; erdpy wallet pem-address-hex $WALLET)
}

account_of() {
    local ADDRESS=$(address_of $1)
    (set -x; erdpy account get --address $ADDRESS)
}

balance_of() {
    local ADDRESS=$(address_of $1)
    (set -x; erdpy account get --balance --address $ADDRESS)
}

load_var() {
    local VAR_NAME=$1
    local VAR_VALUE="$(erdpy data load --key=${VAR_NAME})"
    export -n "${VAR_NAME}=$VAR_VALUE"
}

save_var() {
    local VAR_NAME=$1
    local VAR_VALUE=$2
    export -n "${VAR_NAME}=$VAR_VALUE"
    erdpy data store --key=${VAR_NAME} --value=${VAR_VALUE}
}

deploy_contract() {
    local CONTRACT_NAME_LOWERCASE=$1
    local OWNER_LOWERCASE=$2

    local CONTRACT_NAME_UPPERCASE=${1^^}
    local OWNER_UPPERCASE=${2^^}
    local OWNER_WALLET=$(wallet_of ${OWNER_LOWERCASE})
    local OUTFILE="${CONTRACT_NAME_LOWERCASE}-${OWNER_LOWERCASE}.interaction.json"
    local ADDRESS_VAR_NAME="${CONTRACT_NAME_UPPERCASE}_${OWNER_UPPERCASE}_ADDRESS"
    local DEPLOY_TRANSACTION_VAR_NAME="${CONTRACT_NAME_UPPERCASE}_${OWNER_UPPERCASE}_DEPLOY_TRANSACTION"
    
    echo ""
    echo "Deploying '${CONTRACT_NAME_LOWERCASE}' owned by '${OWNER_LOWERCASE}'..."
    echo ""

    (set -x; erdpy --verbose contract deploy --bytecode="${CONTRACT_NAME_LOWERCASE}/output/${CONTRACT_NAME_LOWERCASE}.wasm" --recall-nonce \
        --pem="${OWNER_WALLET}" --gas-limit=1000000000 \
        --send --outfile="${OUTFILE}" \
        || return)

    local RESULT_ADDRESS=$(erdpy data parse --file="${OUTFILE}" --expression="data['emitted_tx']['address']")
    local RESULT_TRANSACTION=$(erdpy data parse --file="${OUTFILE}" --expression="data['emitted_tx']['hash']")

    save_var ${ADDRESS_VAR_NAME} ${RESULT_ADDRESS}
    save_var ${DEPLOY_TRANSACTION_VAR_NAME} ${RESULT_TRANSACTION}

    echo ""
    echo "Deployed '${CONTRACT_NAME_LOWERCASE}' owned by '${OWNER_LOWERCASE}' with:"
    echo "  \${${ADDRESS_VAR_NAME}} == ${!ADDRESS_VAR_NAME}"
    echo "  \${${DEPLOY_TRANSACTION_VAR_NAME}} == ${!DEPLOY_TRANSACTION_VAR_NAME}"
    echo ""
}

build_contract() {
    CONTRACT_NAME=$1
    (set -x; erdpy --verbose contract build "${CONTRACT_NAME}")
}

build() {
    build_contract oracle
    build_contract aggregator
    build_contract exchange
}

deploy() {
    # Oracles:

    # Bob: owns oracle-bob smart contract
    deploy_contract oracle bob

    # Dan: owns oracle-dan smart contract
    deploy_contract oracle dan

    # Frank: owns oracle-frank smart contract
    deploy_contract oracle frank

    # Aggregator:
    # Alice: owns the aggregator-alice smart contract
    deploy_contract aggregator alice

    # Exchange:
    # Eve: owns the exchange-eve smart contract
    deploy_contract exchange eve
}

# example calls:
# call exchange eve get_latest_data 0

call() {
    local CONTRACT_ADDRESS=$1
    local TRANSACTION_OWNER=$2
    local FUNCTION_NAME=$3
    local FUNCTION_ARGUMENTS=$4

    local WALLET=$(wallet_of $TRANSACTION_OWNER)
    (set -x; erdpy --verbose contract call ${CONTRACT_ADDRESS} --recall-nonce --pem=${WALLET} \
        --gas-limit=5000000 --function="${FUNCTION_NAME}" --arguments ${FUNCTION_ARGUMENTS} --send)
}

call_testnet() {
    local CONTRACT_NAME=$1
    local CONTRACT_OWNER=$2
    local TRANSACTION_OWNER=$3
    local FUNCTION_NAME=$3
    local FUNCTION_ARGUMENTS=$4

    local ADDRESS_VAR="${CONTRACT_NAME^^}_${CONTRACT_OWNER^^}_ADDRESS"
    local CONTRACT_ADDRESS=${!ADDRESS_VAR}
    if [ -z "$CONTRACT_ADDRESS" ]; then
        echo "Contract '${CONTRACT_NAME}-${CONTRACT_OWNER}' not deployed";
        return 1;
    fi
    echo "Calling ${CONTRACT_NAME}-${CONTRACT_OWNER}.${FUNCTION_NAME}..."
    call $CONTRACT_ADDRESS $TRANSACTION_OWNER $FUNCTION_NAME $FUNCTION_ARGUMENTS
}

from_ascii_to_hex() {
    local TEXT=$1
    echo ${TEXT} | tr -d '\n' | xxd -pu -
}

from_number_to_hex() {
    local NUMBER=$1
    local HEX_NUMBER=$(printf '%x' $NUMBER)
    local HEX_NUMBER_LENGTH=${#HEX_NUMBER}
    local PADDED_LENGTH=$(($HEX_NUMBER_LENGTH % 2 + $HEX_NUMBER_LENGTH))
    printf "%0${PADDED_LENGTH}x" $NUMBER
}

issue_tokens() {
    local TOKEN_OWNER=$1
    local TOKEN_NAME=$2
    local TOKEN_TICKER=$3
    local TOKEN_INITIAL_SUPPLY=$4
    local TOKEN_NUMBER_OF_DECIMALS=$5

    local OWNER_WALLET=$(wallet_of $TOKEN_OWNER)
    local TOKEN_NAME_HEX=$(from_ascii_to_hex $TOKEN_NAME)
    local TOKEN_TICKER_HEX=$(from_ascii_to_hex $TOKEN_TICKER)
    local TOKEN_INITIAL_SUPPLY_HEX=$(from_number_to_hex $TOKEN_INITIAL_SUPPLY)
    local TOKEN_NUMBER_OF_DECIMALS_HEX=$(from_number_to_hex $TOKEN_NUMBER_OF_DECIMALS)
    local TRANSACTION_DATA="issue@$TOKEN_NAME_HEX@$TOKEN_TICKER_HEX@$TOKEN_INITIAL_SUPPLY_HEX@$TOKEN_NUMBER_OF_DECIMALS_HEX"
    (set -x; erdpy --verbose tx new ${CONTRACT_ADDRESS} --recall-nonce --pem=${OWNER_WALLET} \
        --receiver $METACHAIN_ADDRESS --value=5000000000000000000 --gas-limit=100000000 \
        --data $TRANSACTION_DATA --send)
}

check_issued_tokens() {
    call $METACHAIN_ADDRESS eve getAllESDTTokens 0
}
