# How to use market multiple creators template

1. Init the contract with a list of co-founders: address & share. The transaction creator account must be included in the list

2. Mint nfts: Mint 721 & 1155. Same input as market implementation, only add contract address. Only co-founders can mint. When mint, default approve all for the marketplace that is responsible for minting

3. Sell & ask nft: The multi creator contract will need to approve the caller (one of the co-founder). Then the caller can use the marketplace directly to sell the nft

4. Receieve revenue: money will be sent to the contract => anyone can invoke the share revenue to distribute revenue to the co-founders.

5. When changing the contract, need to revoke those that will no longer in the co-founder list. Also, One of the co-founders can change the royalty creator of the contract.
