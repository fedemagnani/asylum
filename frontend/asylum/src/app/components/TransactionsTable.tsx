
"use client";
import { useEffect, useState } from 'react';
import './TransactionsTable.css'; 

// We reduce the dimension of the font 

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

const TransactionsTable = () => {
    const [data, setData] = useState<TransactionDatasetObject[]>([]);

    useEffect(() => { //triggered with rendering of component
        const fetchData = async () => {
            const response = await fetch('http://localhost:80/transactions');
            const data: TransactionDatasetObject[] = await response.json();
            setData(data);
        };

        fetchData();
        const intervalId = setInterval(fetchData, 5000); // Fetch data every 5 seconds

        return () => clearInterval(intervalId); // Clean up on unmount
    }, []);

    return (
        <table>
            <thead>
                <tr>
                    <th>Timestamp</th>
                    <th>Hash</th>
                    <th>From Address</th>
                    {/* <th>From Entity ID</th> */}
                    <th>From Entity Name</th>
                    {/* <th>From Entity Label</th> */}
                    {/* <th>From Entity Type</th> */}
                    {/* <th>From Entity Twitter</th> */}
                    <th>To Address</th>
                    {/* <th>To Entity ID</th> */}
                    <th>To Entity Name</th>
                    {/* <th>To Entity Label</th> */}
                    {/* <th>To Entity Type</th> */}
                    {/* <th>To Entity Twitter</th> */}
                    <th>Token Address</th>
                    <th>Chain</th>
                    <th>Block Number</th>
                    {/* <th>Block Hash</th> */}
                </tr>
            </thead>
            <tbody>
                {data.map((transaction, index) => (
                    <tr key={index}>
                        <td>{transaction.block_timestamp}</td>
                        <td>{transaction.hash}</td>
                        <td>
                            <a href={`https://debank.com/profile/${transaction.from_address}`}>
                                {transaction.from_address}
                            </a>
                        </td>
                        {/* <td>{transaction.from_entity_id}</td> */}
                        <td>
                            <a href={`https://debank.com/profile/${transaction.from_address}`}>
                                {transaction.from_entity_name}
                            </a>
                        </td>
                        {/* <td>{transaction.from_entity_label}</td> */}
                        {/* <td>{transaction.from_entity_type}</td> */}
                        {/* <td>{transaction.from_entity_twitter}</td> */}
                        <td>
                            <a href={`https://debank.com/profile/${transaction.to_address}`}>
                                {transaction.to_address}
                            </a>
                        </td>
                        {/* <td>{transaction.to_entity_id}</td> */}
                        <td>
                            <a href={`https://debank.com/profile/${transaction.to_address}`}>
                                {transaction.to_entity_name}
                            </a>
                        </td>
                        {/* <td>{transaction.to_entity_label}</td> */}
                        {/* <td>{transaction.to_entity_type}</td> */}
                        {/* <td>{transaction.to_entity_twitter}</td> */}
                        <td>
                            <a href={`https://debank.com/profile/${transaction.token_address}`}>
                                {transaction.token_address}
                            </a>
                        </td>
                        <td>{transaction.chain}</td>
                        <td>{transaction.block_number}</td>
                        {/* <td>{transaction.block_hash}</td> */}
                    </tr>
                ))}
            </tbody>
        </table>
    );
};

export default TransactionsTable;