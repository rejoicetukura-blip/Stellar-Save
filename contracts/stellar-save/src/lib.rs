#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct StellarSaveContract;

#[contractimpl]
impl StellarSaveContract {
    pub fn hello(env: Env) -> soroban_sdk::Symbol {
        soroban_sdk::symbol_short!("hello")
    }
}
