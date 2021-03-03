USERS="../my-testnet/testnet/wallets/users"
METACHAIN_ADDRESS="erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u"
ISSUED_TOKENS=1000000000
SOME_TOKENS=1000000

wallet_of() {
    local NAME=$1
    echo "${USERS}/${NAME}.pem"
}

address_of() {
    local WALLET=$(wallet_of $1)
    (set -x; erdpy wallet pem-address $WALLET)
}

hex_address_of() {
    local WALLET=$(wallet_of $1)
    (set -x; erdpy wallet pem-address-hex $WALLET)
}

bech_to_hex() {
    (set -x; erdpy wallet bech32 --decode $1)
}

account_of() {
    local ADDRESS=$(hex_address_of $1)
    (set -x; erdpy account get --address $ADDRESS)
}

balance_of() {
    local ADDRESS=$(hex_address_of $1)
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
    local OTHER_ARGUMENTS=${@:3}

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
        --pem="${OWNER_WALLET}" --gas-limit=1000000000 $OTHER_ARGUMENTS \
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

call_sc_address() {
    local CONTRACT_ADDRESS=$1
    local TRANSACTION_OWNER=$2
    local OTHER_ARGUMENTS=${@:3}

    local WALLET=$(wallet_of $TRANSACTION_OWNER)
    (set -x; erdpy --verbose contract call ${CONTRACT_ADDRESS} --recall-nonce --pem=${WALLET} \
        --gas-limit=500000000 ${OTHER_ARGUMENTS} --send)
}

# example calls:
# call_sc exchange eve eve --function get_latest_data

call_sc() {
    local CONTRACT_NAME=$1
    local CONTRACT_OWNER=$2
    local TRANSACTION_OWNER=$3
    local OTHER_ARGUMENTS=${@:4}

    local ADDRESS_VAR="${CONTRACT_NAME^^}_${CONTRACT_OWNER^^}_ADDRESS"
    local CONTRACT_ADDRESS=${!ADDRESS_VAR}
    if [ -z "$CONTRACT_ADDRESS" ]; then
        echo "Contract '${CONTRACT_NAME}-${CONTRACT_OWNER}' not deployed";
        return 1;
    fi
    echo "Calling ${CONTRACT_NAME}-${CONTRACT_OWNER}.${FUNCTION_NAME}..."
    call_sc_address $CONTRACT_ADDRESS $TRANSACTION_OWNER $OTHER_ARGUMENTS
}

ascii_to_hex() {
    local TEXT=$1
    echo ${TEXT} | tr -d '\n' | xxd -pu -
}

ascii_arg() {
    local TEXT=$1
    echo "0x$(ascii_to_hex $TEXT)"
}

number_to_hex() {
    local NUMBER=$1
    local HEX_NUMBER=$(printf '%x' $NUMBER)
    local HEX_NUMBER_LENGTH=${#HEX_NUMBER}
    local PADDED_LENGTH=$(($HEX_NUMBER_LENGTH % 2 + $HEX_NUMBER_LENGTH))
    printf "%0${PADDED_LENGTH}x" $NUMBER
}

address_arg() {
    local ADDRESS=$1
    echo "0x$(bech_to_hex $ADDRESS)"
}

number_arg() {
    local NUMBER=$1
    echo "0x$(number_to_hex $NUMBER)"
}

issue_tokens() {
    local TOKEN_OWNER=$1
    local TOKEN_NAME=$2
    local TOKEN_TICKER=$3
    local TOKEN_INITIAL_SUPPLY=$4
    local TOKEN_NUMBER_OF_DECIMALS=$5

    local OWNER_WALLET=$(wallet_of $TOKEN_OWNER)
    local TOKEN_NAME_HEX=$(ascii_to_hex $TOKEN_NAME)
    local TOKEN_TICKER_HEX=$(ascii_to_hex $TOKEN_TICKER)
    local TOKEN_INITIAL_SUPPLY_HEX=$(number_to_hex $TOKEN_INITIAL_SUPPLY)
    local TOKEN_NUMBER_OF_DECIMALS_HEX=$(number_to_hex $TOKEN_NUMBER_OF_DECIMALS)
    local TRANSACTION_DATA="issue@$TOKEN_NAME_HEX@$TOKEN_TICKER_HEX@$TOKEN_INITIAL_SUPPLY_HEX@$TOKEN_NUMBER_OF_DECIMALS_HEX"
    (set -x; erdpy --verbose tx new ${CONTRACT_ADDRESS} --recall-nonce --pem=${OWNER_WALLET} \
        --receiver $METACHAIN_ADDRESS --value=5000000000000000000 --gas-limit=100000000 \
        --data $TRANSACTION_DATA --send)
}

send_egld() {
    local OWNER=$1
    local OTHER_ARGUMENTS=${@:2}

    local OWNER_WALLET=$(wallet_of $OWNER)
    (set -x; erdpy --verbose tx new $OTHER_ARGUMENTS --recall-nonce --pem=${OWNER_WALLET} --gas-limit=100000 --send)
}

send_esdt() {
    local SOURCE=$1
    local DESTINATION=$2
    local TOKEN_ID=$3
    local AMOUNT=$4

    local SOURCE_WALLET=$(wallet_of $SOURCE)
    local TOKEN_ID_HEX=$(ascii_to_hex $TOKEN_ID)
    local AMOUNT_HEX=$(number_to_hex $AMOUNT)
    (set -x; erdpy --verbose tx new --recall-nonce --pem=${SOURCE_WALLET} --receiver $(address_of $DESTINATION) \
        --gas-limit=400000 --data "ESDTTransfer@$TOKEN_ID_HEX@$AMOUNT_HEX" --send)
}

send_esdt_with_call() {
    local SOURCE=$1
    local DESTINATION_ADDRESS=$2
    local TOKEN_ID=$3
    local AMOUNT=$4
    local OTHER_ARGUMENTS=${@:5}

    local SOURCE_WALLET=$(wallet_of $SOURCE)
    local TOKEN_ID_HEX=$(ascii_to_hex $TOKEN_ID)
    local AMOUNT_HEX=$(number_to_hex $AMOUNT)
    (set -x; erdpy --verbose tx new --recall-nonce --pem=${SOURCE_WALLET} --receiver $DESTINATION_ADDRESS \
        --gas-limit=500000000 --data "ESDTTransfer@$TOKEN_ID_HEX@$AMOUNT_HEX@$OTHER_ARGUMENTS" --send)
}

check_issued_tokens() {
    call $METACHAIN_ADDRESS eve getAllESDTTokens 0
}

step_1_issue_tokens() {
    issue_tokens eve tokenA TOKA $ISSUED_TOKENS 6
    sleep 6
    issue_tokens eve tokenB TOKB $ISSUED_TOKENS 12
    sleep 6
    call_sc_address $METACHAIN_ADDRESS eve --function getAllESDTTokens
}

# note: token names can be obtained from the results of the getAllESDTTokens call from step 1
# using the transaction hash with http://localhost:7950/transaction/:transaction_hash?withResults=true
# example (note - replace the token names with those from your own testnet):
# step_2_configure_tokens TOKA-9dabbe TOKB-4a18f6
step_2_configure_tokens() {
    TOKEN_A=$1
    TOKEN_B=$2
}

step_3_deploy_sc() {
    # clean-up
    rm ./*.interaction.json

    # Oracles: oracle-bob, oracle-dan, oracle-frank
    deploy_contract oracle bob
    deploy_contract oracle dan
    deploy_contract oracle frank

    # Aggregator: aggregator-alice
    local TOKEN_ID="EGLD"
    local PAYMENT_AMOUNT=1000
    local TIMEOUT=1000
    local MIN_SUBMISSION_VALUE=10
    local MAX_SUBMISSION_VALUE=1000000000
    local DECIMALS=3
    local DESCRIPTION="${TOKEN_A}/${TOKEN_B}"
    deploy_contract aggregator alice \
        --arguments $(ascii_arg $TOKEN_ID) $(number_arg $PAYMENT_AMOUNT) $(number_arg $TIMEOUT) \
        $(number_arg $MIN_SUBMISSION_VALUE) $(number_arg $MAX_SUBMISSION_VALUE) $(number_arg $DECIMALS) $(ascii_arg $DESCRIPTION)

    # Exchange: exchange-eve
    deploy_contract exchange eve --arguments $(address_arg ${AGGREGATOR_ALICE_ADDRESS})
}

step_4_prepare_aggregator() {
    # add_funds
    call_sc aggregator alice alice --function add_funds --value 6000000

    sleep 6

    # change_oracles
    local ORACLE_ADDRESSES="0x$(bech_to_hex ${ORACLE_BOB_ADDRESS})$(bech_to_hex ${ORACLE_DAN_ADDRESS})$(bech_to_hex ${ORACLE_FRANK_ADDRESS})"
    local OWNER_ADDRESSES="0x$(hex_address_of bob)$(hex_address_of dan)$(hex_address_of frank)"
    call_sc aggregator alice alice \
        --function change_oracles --arguments 0x $ORACLE_ADDRESSES $OWNER_ADDRESSES 0x03 0x03 0x

    sleep 6

    # set_requester_permissions
    local REQUESTER_ADDRESS_ARG=0x$(bech_to_hex $(address_of grace))
    call_sc aggregator alice alice --function set_requester_permissions --arguments $REQUESTER_ADDRESS_ARG 0x01 0x

    sleep 6

    # request round 1
    call_sc aggregator alice grace --function request_new_round
}

step_5_prepare_exchange() {
    send_esdt_with_call eve ${EXCHANGE_EVE_ADDRESS} $TOKEN_A $SOME_TOKENS "$(ascii_to_hex deposit)"
    
    sleep 6

    send_esdt_with_call eve ${EXCHANGE_EVE_ADDRESS} $TOKEN_B $SOME_TOKENS "$(ascii_to_hex deposit)"
}

step_6_send_funds_to_other_users() {
    send_esdt eve heidi $TOKEN_A $SOME_TOKENS
    
    sleep 6

    send_esdt eve ivan $TOKEN_B $SOME_TOKENS
}

oracle_submit() {
    local OWNER=$1
    local ROUND=$2
    local SUBMISSION=$3

    call_sc oracle $OWNER $OWNER --function submit \
        --arguments $(address_arg ${AGGREGATOR_ALICE_ADDRESS}) $(number_arg $ROUND) $(number_arg $SUBMISSION)
}

step_7_send_round_1() {
    oracle_submit frank 1 1195
    oracle_submit bob 1 1200
    oracle_submit dan 1 1205
}

exchange_tokens() {
    local OWNER=$1
    local TOKEN=$2
    local AMOUNT=$3
    local TARGET_TOKEN=$4

    send_esdt_with_call $OWNER $EXCHANGE_EVE_ADDRESS $TOKEN $AMOUNT \
        "$(ascii_to_hex exchange)@$(ascii_to_hex $TARGET_TOKEN)"
}

step_8_exchange_tokens() {
    exchange_tokens heidi $TOKEN_A 1000 $TOKEN_B
    # heidi should now have 1200 of token B

    sleep 6

    exchange_tokens ivan $TOKEN_B 1000 $TOKEN_A
    # ivan should now have 833 of token A
}

step_9_send_round_2() {
    oracle_submit frank 2 1295
    oracle_submit bob 2 1300
    oracle_submit dan 2 1305
}
