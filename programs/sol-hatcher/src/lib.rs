use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        create_metadata_accounts_v3,
        mpl_token_metadata::{accounts::Metadata as MetadataAccount, types::DataV2},
        CreateMetadataAccountsV3, Metadata,
    },
    token::{self, mint_to, Mint, MintTo, Token, TokenAccount, Transfer},
};
use solana_program::{pubkey, pubkey::Pubkey};

declare_id!("8EgjF9Ema9VpR2XFqsPt591n5rvgBDJqB1dGHdVJhFm9");

const ADMIN_PUBKEY: Pubkey = pubkey!("4wvkHZTw9HiV23zko2FogZAU5sjErwE34dKMSz2x1P93");

// const SUBMIT_FEE: u64 = 10000000; // 1 SOL
const TOKEN_NAME: &str = "SOL Hatch Token";
const TOKEN_SYMBOL: &str = "HAT";
const URI: &str = "https://arweave.net/123456";

#[program]
pub mod sol_hatcher {

    use solana_program::entrypoint::ProgramResult;

    use super::*;

    pub fn initialize_data(_ctx: Context<Initialize>) -> Result<()> {
        // init config and leaderboard
        let seeds = b"hatcherToken";
        let bump = _ctx.bumps.hatcher_token_mint;
        let signer: &[&[&[u8]]] = &[&[seeds, &[bump]]];

        // On-chain token metadata for the mint
        let data_v2 = DataV2 {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            uri: URI.to_string(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        // CPI Context
        let cpi_ctx = CpiContext::new_with_signer(
            _ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: _ctx.accounts.metadata_account.to_account_info(), // the metadata account being created
                mint: _ctx.accounts.hatcher_token_mint.to_account_info(), // the mint account of the metadata account
                mint_authority: _ctx.accounts.hatcher_token_mint.to_account_info(), // the mint authority of the mint account
                update_authority: _ctx.accounts.hatcher_token_mint.to_account_info(), // the update authority of the metadata account
                payer: _ctx.accounts.admin.to_account_info(), // the payer for creating the metadata account
                system_program: _ctx.accounts.system_program.to_account_info(), // the system program account, required when creating new accounts
                rent: _ctx.accounts.rent.to_account_info(), // the rent sysvar account
            },
            signer, // pda signer
        );

        create_metadata_accounts_v3(
            cpi_ctx, // cpi context
            data_v2, // token metadata
            true,    // is_mutable
            true,    // update_authority_is_signer
            None,    // collection details
        )?;

        // init hatch data
        let hatch_data = &mut _ctx.accounts.hatch_data;
        hatch_data.leaderboard = [].to_vec();
        hatch_data.nonce = _ctx.bumps.vault_signer;

        Ok(())
    }

    pub fn update_leaderboard(
        _ctx: Context<UpdateLeaderboard>,
        new_leaderboard: Vec<LeaderboardItem>,
    ) -> Result<()> {
        // 1. Update leaderboard
        let _leaderboard = &mut _ctx.accounts.hatch_data.leaderboard;
        _leaderboard.clear();
        // 2. Mint award token to successful creators
        for item in new_leaderboard {
            _leaderboard.push(item);
        }
        // PDA seeds and bump to "sign" for CPI
        let seeds = b"hatcherToken";
        let bump = _ctx.bumps.hatcher_token_mint;
        let signer: &[&[&[u8]]] = &[&[seeds, &[bump]]];

        // CPI Context
        let cpi_ctx = CpiContext::new_with_signer(
            _ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: _ctx.accounts.hatcher_token_mint.to_account_info(), // mint account of token to mint
                to: _ctx.accounts.winner_token_account.to_account_info(),
                // to: AccountInfo::new(_leaderboard[0].creator),
                // to: _ctx.accounts.(_leaderboard[0].creator) , // _ctx.accounts.player_token_account.to_account_info(), // player token account to mint to
                authority: _ctx.accounts.hatcher_token_mint.to_account_info(), // pda is used as both address of mint and mint authority
            },
            signer, // pda signer
        );
        let amount = (1000000u64)
            .checked_mul(10u64.pow(_ctx.accounts.hatcher_token_mint.decimals as u32))
            .unwrap();
        mint_to(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn deposit_token(_ctx: Context<DepositToken>, amount: u64) -> Result<()> {
        let cpi_accounts = Transfer {
            from: _ctx.accounts.user_token_account.to_account_info(),
            to: _ctx.accounts.vault_token_account.to_account_info(),
            authority: _ctx.accounts.user.to_account_info(),
        };
        let cpi_program = _ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        _ctx.accounts.user_balance_account.amount += amount;

        Ok(())
    }

    pub fn withdraw_token(_ctx: Context<WithdrawToken>, amount: u64) -> ProgramResult {
        // transfer
        // token account

        if _ctx.accounts.user_balance_account.amount < amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // let seeds = &[
        //     &_ctx.accounts.token_program.to_account_info().key.to_bytes()[..],
        //     &[_ctx.accounts.hatch_data.nonce],
        // ];

        // let signer = &[&seeds[..]];

        let seeds = b"vaultSigner";
        let bump = _ctx.bumps.vault_signer;
        // let bump = _ctx.accounts.hatch_data.nonce; // _ctx.bumps.vault;
        let signer: &[&[&[u8]]] = &[&[seeds, &[bump]]];

        let cpi_accounts = Transfer {
            from: _ctx.accounts.vault_token_account.to_account_info(),
            to: _ctx.accounts.user_token_account.to_account_info(),
            authority: _ctx.accounts.vault_signer.to_account_info(),
        };

        let cpi_program = _ctx.accounts.token_program.to_account_info();

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct UpdateLeaderboard<'info> {
    #[account(
      mut,
      address = ADMIN_PUBKEY
  )]
    pub admin: Signer<'info>,

    #[account(
      mut,
      seeds = [b"hatchData", admin.key().as_ref()],
      bump,
    )]
    pub hatch_data: Account<'info, HatchData>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,

    #[account(
        mut,
        seeds = [b"hatcherToken"],
        bump,
    )]
    pub hatcher_token_mint: Account<'info, Mint>,

    /// CHECK: winnerAccount
    pub winner_account: UncheckedAccount<'info>,

    #[account(
      init_if_needed,
      payer = admin,
      associated_token::mint = hatcher_token_mint,
      associated_token::authority = winner_account
    )]
    pub winner_token_account: Account<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct DepositToken<'info> {
    #[account(mut)]
    hatch_data: Account<'info, HatchData>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 8 + 32,
        seeds = [b"userBalance", user.key().as_ref()],
        bump,
    )]
    pub user_balance_account: Account<'info, UserBalance>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawToken<'info> {
    #[account(mut)]
    hatch_data: Account<'info, HatchData>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        seeds = [b"vaultSigner"],
        bump
    )]
    pub vault_signer: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,

    #[account(
        mut,
        seeds = [b"userBalance", user.key().as_ref()],
        bump,
    )]
    pub user_balance_account: Account<'info, UserBalance>,
}

/**
 * Struct to store leaderboard data
 * 1 + 32 + 4 + (8 + 8 + 32) * 10 = 517
 */
#[account]
pub struct HatchData {
    pub nonce: u8,
    pub token_account: Pubkey,
    pub leaderboard: Vec<LeaderboardItem>,
}

// Define struct of an item in the leaderboard
#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct LeaderboardItem {
    pub agent_id: u64,
    pub creator: Pubkey,
    pub score: u64,
}

#[account]
pub struct UserBalance {
    pub user: Pubkey,
    pub amount: u64,
}

// create token mint
#[derive(Accounts)]
pub struct Initialize<'info> {
    // Use ADMIN_PUBKEY as constraint, only the specified admin can invoke this instruction
    #[account(
          mut,
          address = ADMIN_PUBKEY
      )]
    pub admin: Signer<'info>,

    // The PDA is both the address of the mint account and the mint authority
    #[account(
          init,
          seeds = [b"hatcherToken"],
          bump,
          payer = admin,
          mint::decimals = 9,
          mint::authority = hatcher_token_mint,
      )]
    pub hatcher_token_mint: Account<'info, Mint>,

    ///CHECK: Using "address" constraint to validate metadata account address, this account is created via CPI in the instruction
    #[account(
          mut,
          address = MetadataAccount::find_pda(&hatcher_token_mint.key()).0,
      )]
    pub metadata_account: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    #[account(
        init,
        payer = admin,
        space = 1 + 32 + 4 + (8 + 8 + 32) * 10,
        seeds = [b"hatchData", admin.key().as_ref()],
        bump,
      )]
    pub hatch_data: Account<'info, HatchData>,

    /// CHECK: init a singer info
    #[account(
        seeds = [b"vaultSigner"],
        bump
    )]
    pub vault_signer: UncheckedAccount<'info>,
}
