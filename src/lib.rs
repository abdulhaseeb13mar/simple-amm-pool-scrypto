use scrypto::prelude::*;

blueprint! {
    struct Radiswap {
        lp_resource_address: ResourceAddress,
        lp_mint_badge: Vault,
        a_pool: Vault,
        b_pool: Vault,
        fee: Decimal,
        lp_per_asset_ratio: Decimal,
    }

    impl Radiswap {

        pub fn instantiate_pool(a_tokens: Bucket, b_tokens: Bucket, lp_initial_supply: Decimal, lp_symbol: String, lp_name: String, lp_url: String, fee: Decimal) -> (ComponentAddress, Bucket) {
            assert!(!a_tokens.is_empty() && ! b_tokens.is_empty());
            assert!(fee >= dec!("0") && fee <= dec!("1"), "invalid fee in thousands");
            let lp_mint_badge = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", "LP Token Mint Auth")
                .initial_supply(1);
            let lp_resource_address = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_MAXIMUM)
                .metadata("symbol", lp_symbol)
                .metadata("name", lp_name)
                .metadata("url", lp_url)
                .mintable(rule!(require(lp_mint_badge.resource_address())), LOCKED)
                .burnable(rule!(require(lp_mint_badge.resource_address())), LOCKED)
                .no_initial_supply();
            
            let lp_tokens = lp_mint_badge.authorize(|| {
                borrow_resource_manager!(lp_resource_address).mint(lp_initial_supply)
            });
           
            let lp_per_asset_ratio = lp_initial_supply / (a_tokens.amount() * b_tokens.amount());


            let radiswap = Self {
                lp_resource_address,
                lp_mint_badge: Vault::with_bucket(lp_mint_badge),
                a_pool: Vault::with_bucket(a_tokens),
                b_pool: Vault::with_bucket(b_tokens),
                fee,
                lp_per_asset_ratio,
            }
            .instantiate()
            .globalize();

            (radiswap, lp_tokens)
        }

        pub fn add_liquidity(&mut self, mut a_tokens: Bucket, mut b_tokens: Bucket) -> (Bucket, Bucket) {
            let lp_resource_manager = borrow_resource_manager!(self.lp_resource_address);

              
            let (supply_to_mint, remainder) = if lp_resource_manager.total_supply() == 0.into() {
                let supply_to_mint = self.lp_per_asset_ratio * a_tokens.amount() * b_tokens.amount();
                self.a_pool.put(a_tokens.take(a_tokens.amount()));
                self.b_pool.put(b_tokens);
                (supply_to_mint, a_tokens)
            } else {
                let a_ratio = a_tokens.amount() / self.a_pool.amount();
                let b_ratio = b_tokens.amount() / self.b_pool.amount();

                let (actual_ratio, remainder) = if a_ratio <= b_ratio {
                    self.a_pool.put(a_tokens.take(a_tokens.amount()));
                    self.b_pool.put(b_tokens.take(self.b_pool.amount() * a_ratio));
                    (a_ratio, b_tokens)
                } else {
                    self.b_pool.put(b_tokens.take(b_tokens.amount()));
                    self.a_pool.put(a_tokens.take(self.a_pool.amount() * b_ratio));
                    (b_ratio, a_tokens)
                };

                (lp_resource_manager.total_supply() * actual_ratio, remainder)
            };

            let lp_tokens = self.lp_mint_badge.authorize(|| {
                lp_resource_manager.mint(supply_to_mint)
            });

            (lp_tokens, remainder)
        }
    
        pub fn remove_liquidity(&mut self, lp_tokens: Bucket) -> (Bucket, Bucket) {
            assert!(self.lp_resource_address == lp_tokens.resource_address(), "Wrong Lp Tokens Passed");

            let lp_resource_manager = borrow_resource_manager!(self.lp_resource_address);

            let share = lp_tokens.amount() / lp_resource_manager.total_supply();

            let a_withdrawn = self.a_pool.take(self.a_pool.amount() * share);
            let b_withdrawn = self.b_pool.take(self.b_pool.amount() * share);

            self.lp_mint_badge.authorize(|| {
                lp_tokens.burn()
            });

            (a_withdrawn, b_withdrawn)
        } 
    
        pub fn swap(&mut self, input_tokens: Bucket) -> () {

            assert!(
                input_tokens.resource_address() == self.a_pool.resource_address() || 
                input_tokens.resource_address() == self.b_pool.resource_address(),
                "Wrong token input"
            );

            // let lp_resource_manager = borrow_resource_manager!(self.lp_resource_address);

            // let fee_amount = input_tokens.amount() * self.fee;
        }
    
    }
}
