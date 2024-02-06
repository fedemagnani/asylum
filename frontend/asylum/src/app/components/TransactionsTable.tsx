

import { useEffect, useState } from 'react';
import './TransactionsTable.css'; 
import {fetchData} from '../utils';
import path from 'path';
// We reduce the dimension of the font 



const TransactionsTable: React.FC<{data: any[]}>  = ({data}) => {

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
                            <a  target="_blank" rel="noopener noreferrer" href={`https://debank.com/profile/${transaction.from_address}`}>
                                {transaction.from_address}
                            </a>
                        </td>
                        {/* <td>{transaction.from_entity_id}</td> */}
                        <td>
                            <a  target="_blank" rel="noopener noreferrer" href={`https://debank.com/profile/${transaction.from_address}`}>
                                {transaction.from_entity_name}
                            </a>
                        </td>
                        {/* <td>{transaction.from_entity_label}</td> */}
                        {/* <td>{transaction.from_entity_type}</td> */}
                        {/* <td>{transaction.from_entity_twitter}</td> */}
                        <td>
                            <a  target="_blank" rel="noopener noreferrer" href={`https://debank.com/profile/${transaction.to_address}`}>
                                {transaction.to_address}
                            </a>
                        </td>
                        {/* <td>{transaction.to_entity_id}</td> */}
                        <td>
                            <a  target="_blank" rel="noopener noreferrer" href={`https://debank.com/profile/${transaction.to_address}`}>
                                {transaction.to_entity_name}
                            </a>
                        </td>
                        {/* <td>{transaction.to_entity_label}</td> */}
                        {/* <td>{transaction.to_entity_type}</td> */}
                        {/* <td>{transaction.to_entity_twitter}</td> */}
                        <td>
                            <a  target="_blank" rel="noopener noreferrer" href={`https://debank.com/profile/${transaction.token_address}`}>
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

export {TransactionsTable};