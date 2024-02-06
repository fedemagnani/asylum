
interface TransactionDatasetObject {
    hash: string,
    from_address: string,
    from_entity_id: string,
    from_entity_name: string,
    from_entity_label: string,
    from_entity_type: string,
    from_entity_twitter: string,
    to_address: string,
    to_entity_id: string,
    to_entity_name: string,
    to_entity_label: string,
    to_entity_type: string,
    to_entity_twitter: string,
    token_address: string,
    chain: string,
    block_number: number,
    block_timestamp: string,
    block_hash: string
}
export type {TransactionDatasetObject};