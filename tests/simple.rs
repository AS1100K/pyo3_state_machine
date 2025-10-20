use pyo3_state_machine::py_state_machine;

pub trait AuthStatus {}

pub struct AuthPending;
impl AuthStatus for AuthPending {}

pub struct Authenticated;
impl AuthStatus for Authenticated {}

#[py_state_machine(PasswordManagerAuthPending, B = Authenticated, A = AuthPending)]
pub struct PasswordManager<A: AuthStatus, B> {
    api_key: String,
    api_secret: String,
    auth_status: std::marker::PhantomData<A>,
    another_staus: std::marker::PhantomData<B>,
}
