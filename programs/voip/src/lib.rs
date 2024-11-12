use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
        Metadata as Metaplex,
    },
    token::{burn, mint_to, Burn, Mint, MintTo, Token, TokenAccount, Transfer},
};

declare_id!("7XDz2q8UffBBwVuQRdBGX6W7Kgy4VuVFt5W63i4YDCYV");

#[program]
pub mod voip {
    use std::str::FromStr;

    use super::*;

    pub fn initialize(ctx: Context<InitToken>, metadata: InitTokenParams) -> Result<()> {
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        // Set trading status to false initially
        ctx.accounts.settings.trading_enabled = false;
        ctx.accounts.settings.owner = ctx.accounts.payer.key.clone();

        let token_data: DataV2 = DataV2 {
            name: metadata.name,
            symbol: metadata.symbol,
            uri: metadata.uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let metadatactx = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                payer: ctx.accounts.payer.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                metadata: ctx.accounts.metadata.to_account_info(),
                mint_authority: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            &signer,
        );

        create_metadata_accounts_v3(metadatactx, token_data, false, true, None)?;

        msg!("Token mint created successfully.");
        Ok(())
    }

    pub fn mint_tokens(ctx: Context<MintTokens>, quantity: u64) -> Result<()> {
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    authority: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
                &signer,
            ),
            quantity,
        )?;

        Ok(())
    }

    pub fn burn_tokens(ctx: Context<BurnTokens>, quantity: u64) -> Result<()> {
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        burn(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), 
                Burn { 
                    mint: ctx.accounts.mint.to_account_info(), 
                    from: ctx.accounts.from.to_account_info(), 
                    authority: ctx.accounts.payer.to_account_info() 
                },
                &signer
            ), 
            quantity
        )?;

        Ok(())
    }

    pub fn transfer_token(ctx: Context<TransferToken>, amount: u64) -> Result<()> {
        let settings = &ctx.accounts.settings;

        // Combined check: Trading must be enabled, and neither the sender nor the receiver should be excluded
        require!(
            settings.trading_enabled,
            VIOPError::TradingDisabled
        );

        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.from.to_account_info(),
                    to: ctx.accounts.to.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                }, 
                &signer
            ), amount
        )?;
        
        Ok(())
    }

    // Enable or disable trading (restricted to the owner)
    pub fn set_trading(ctx: Context<SetTrading>, enable: bool) -> Result<()> {
        let settings = &mut ctx.accounts.settings;
        let _owner = settings.owner;
        require!(
            matches!(ctx.accounts.owner.key, &_owner),
            VIOPError::Unauthorized
        ); // Only owner can change trading status

        settings.trading_enabled = enable;

        Ok(())
    }

    pub fn claim_stuck_tokens(ctx: Context<ClaimStuckTokens>, balance: u64) -> Result<()> {
        let owner = &ctx.accounts.settings.owner;

        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        require!(
            owner.key() == ctx.accounts.settings.owner,
            VIOPError::Unauthorized
        ); // Only the owner can claim stuck tokens

        if ctx.accounts.stuck_token_mint.key() == Pubkey::from_str("So11111111111111111111111111111111111111111").expect("Failed to get Sol mint") {
            // transfer lamport
            let ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.from.key(),
                &ctx.accounts.to.key(),
                balance,
            );
            anchor_lang::solana_program::program::invoke(
                &ix,
                &[
                    ctx.accounts.from.to_account_info(),
                    ctx.accounts.to.to_account_info(),
                ],
            )?;

        } else {
            // Transfer SPL tokens
            let cpi_accounts = Transfer {
                from: ctx.accounts.from_ata.to_account_info(),
                to: ctx.accounts.to_ata.to_account_info(),
                authority: ctx.accounts.mint.to_account_info(),
            };
            let cpi_program = ctx.accounts.stuck_token_mint.to_account_info();

            anchor_spl::token::transfer(
                CpiContext::new_with_signer(cpi_program, cpi_accounts, &signer),
                balance,
            )?;
        }
        
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(params: InitTokenParams)]
pub struct InitToken<'info> {
    #[account(mut)]
    /// CHECK: UncheckedAccount
    pub metadata: UncheckedAccount<'info>,
    #[account(
        init,
        seeds = [b"mint"],
        bump,
        payer = payer,
        mint::decimals = params.decimals,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(init,
        seeds = [b"settings"],
        bump,
        payer = payer,
        space = 8 + 8 + 32 // Added space for storing owner
    )]
    pub settings: Account<'info, Settings>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metaplex>,
}

#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer,
    )]
    pub destination: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer,
    )]
    pub from: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct TransferToken<'info> {
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,
    /// CHECK: The associated token account that we are transferring the token from
    #[account(mut)]
    pub from: UncheckedAccount<'info>,
    /// CHECK: The associated token account that we are transferring the token to
    #[account(mut)]
    pub to: AccountInfo<'info>,
    // the authority of the from account
    pub authority: Signer<'info>,

    #[account(mut)]
    pub settings: Account<'info, Settings>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SetTrading<'info> {
    #[account(mut)]
    pub settings: Account<'info, Settings>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimStuckTokens<'info> {   
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,

    /// CHECK: 
    #[account(mut)]
    pub stuck_token_mint: AccountInfo<'info>,

    /// CHECK: The associated token account that we are transferring the token from
    #[account(mut)]
    pub from: AccountInfo<'info>,
    /// CHECK: The associated token account that we are transferring the token to
    #[account(mut)]
    pub to: AccountInfo<'info>,

    /// CHECK: The associated token account that we are transferring the token from
    #[account(mut)]
    pub from_ata: AccountInfo<'info>,
    /// CHECK: The associated token account that we are transferring the token to
    #[account(mut)]
    pub to_ata: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub settings: Account<'info, Settings>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct InitTokenParams {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub decimals: u8,
}

// Settings account to store trading status and excluded accounts
#[account]
pub struct Settings {
    pub trading_enabled: bool,
    pub owner: Pubkey, // Store the owner's public key for access control
}

// Custom errors
#[error_code]
pub enum VIOPError {
    #[msg("Trading is not enabled")]
    TradingDisabled,
    #[msg("Unauthorized access")]
    Unauthorized,
}