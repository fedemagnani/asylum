import { useEffect, useState } from 'react';
import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import Home from "./components/Home";


const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Asylum",
  description: "Follow the money",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <head>
        <link rel="stylesheet"  href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.3/css/all.min.css"></link>
      </head>
      <body className={inter.className}>
        <Home />
        {children}
      </body>
    </html>
  );
}
