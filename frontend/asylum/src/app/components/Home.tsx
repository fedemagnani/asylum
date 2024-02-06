"use client";
import { useState, useEffect } from 'react';
import { fetchData } from '../utils';
import { TransactionsTable } from './TransactionsTable';
import {Title} from './Title';
import { Spacer } from "./Spacer";
import { TransactionDatasetObject } from './Interfaces';
import { FancyTransactionsTable } from './FancyTransactionsTable';
import { Inter } from "next/font/google";
import {ReloadButton} from './Reload';
const Home = () => {
    const [data, setData] = useState<TransactionDatasetObject[]>([]);
    // useEffect(() => {
    //     fetchData<TransactionDatasetObject[]>('transactions', setData);
    // }, []);
    return (
        <div>
            <Title />
            <Spacer height="2rem" />
            <ReloadButton state_changer={setData} />
            {/* <TransactionsTable data={data} /> */}
            {/* <FancyTransactionsTable/> */}
        </div>
    );
}
    
export default Home;