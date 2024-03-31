use anchor_lang::prelude::*;
use anchor_spl::{
    // associated_token::AssociatedToken,
    metadata::{
        create_metadata_accounts_v3,
        mpl_token_metadata::{accounts::Metadata as MetadataAccount, types::DataV2},
        CreateMetadataAccountsV3, Metadata,
    },
    token::{self, Mint, mint_to, MintTo, Token, TokenAccount, Transfer},
};
use solana_program::{pubkey, pubkey::Pubkey};

declare_id!("8EgjF9Ema9VpR2XFqsPt591n5rvgBDJqB1dGHdVJhFm9");

const ADMIN_PUBKEY: Pubkey = pubkey!("4wvkHZTw9HiV23zko2FogZAU5sjErwE34dKMSz2x1P93");

// const SUBMIT_FEE: u64 = 10000000; // 1 SOL
// const TOKEN_NAME: &str = "SOL Hatch Token";
// const TOKEN_SYMBOL: &str = "HAT";
// const URI: &str = "https://arweave.net/123456";

#[program]
pub mod sol_hatcher {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        // init config and leaderboard

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
                to: _ctx.accounts.winner.to_account_info(),
                // to: _ctx.accounts.(_leaderboard[0].creator) , // _ctx.accounts.player_token_account.to_account_info(), // player token account to mint to
                authority: _ctx.accounts.hatcher_token_mint.to_account_info(), // pda is used as both address of mint and mint authority
            },
            signer, // pda signer
        );
        let amount = (10000u64)
            .checked_mul(10u64.pow(_ctx.accounts.hatcher_token_mint.decimals as u32))
            .unwrap();
        mint_to(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn set_token_config(_ctx: Context<SetTokenConfig>, key_account: Pubkey) -> Result<Pubkey> {
        // check caller identity
        assert_eq!(_ctx.accounts.user.key(), ADMIN_PUBKEY);

        // update in token account in HatchData
        _ctx.accounts.hatch_data.token_account = key_account;

        Ok(key_account)
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

        Ok(())
    }

    pub fn withdraw_token(_ctx: Context<WithdrawToken>, amount: u64) -> Result<()> {
        // transfer
        // token account
        let seeds = &[
            &_ctx.accounts.token_program.to_account_info().key.to_bytes()[..],
            &[_ctx.accounts.hatch_data.nonce],
        ];

        let signer = &[&seeds[..]];

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
      init,
      payer = admin,
      space = 1 + 32 + 4 + (8 + 8 + 32) * 10,
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
    /// CHECK: Winner
    pub winner: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct SetTokenConfig<'info> {
    #[account(mut)]
    hatch_data: Account<'info, HatchData>,
    #[account(mut)]
    pub user: Signer<'info>,
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
    pub vault_signer: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
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

// #[derive(Accounts)]

// create token mint
#[derive(Accounts)]
pub struct Initialize<'info> {
    // Use ADMIN_PUBKEY as constraint, only the specified admin can invoke this instruction
    #[account(
        mut,
        address = ADMIN_PUBKEY
    )]
    pub admin: Signer<'info>,
}
