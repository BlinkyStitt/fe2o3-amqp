mod create;
mod read;
mod update;
mod delete;

pub use create::*;
pub use read::*;
pub use update::*;
pub use delete::*;

pub trait ManageableEntityOperations: Create + Read + Update + Delete {
    
}