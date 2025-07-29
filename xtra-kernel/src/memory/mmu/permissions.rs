
// Simple high-level permissions structure for memory pages in the MMU. This is independent of the
// various architectures and page table formats.

use core::fmt::{ self, Display, Formatter };



#[derive(Default)]
pub struct PermissionsBuilder
{
    readable: bool,
    writable: bool,
    executable: bool,
    user_accessible: bool,
    globally_accessible: bool
}



impl PermissionsBuilder
{
    pub fn readable(mut self) -> Self
    {
        self.readable = true;
        self
    }

    pub fn writable(mut self) -> Self
    {
        self.writable = true;
        self
    }

    pub fn executable(mut self) -> Self
    {
        self.executable = true;
        self
    }

    pub fn user_accessible(mut self) -> Self
    {
        self.user_accessible = true;
        self
    }

    pub fn globally_accessible(mut self) -> Self
    {
        self.globally_accessible = true;
        self
    }

    pub fn build(self) -> Permissions
    {
        Permissions
            {
                readable: self.readable,
                writable: self.writable,
                executable: self.executable,

                user_accessible: self.user_accessible,

                globally_accessible: self.globally_accessible
            }
    }
}



/// The permissions for a page in the memory management unit. This is used to define the access
/// rights for a page when it is mapped into the virtual address space of a process or the kernel.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Permissions
{
    /// Is the page readable?
    pub readable: bool,

    /// Is the page writable?
    pub writable: bool,

    /// Is the page executable?
    pub executable: bool,

    /// Is the page accessible by user space or is it only accessible by the kernel?
    pub user_accessible: bool,

    /// Is the page globally accessible across all address spaces or is it only accessible in the
    /// current address space?
    pub globally_accessible: bool
}



impl Permissions
{
    /// Create a new `Permissions` object with default values. This will create a set of permissions
    /// that allows the page to be read, but not written to or executed.
    ///
    /// We also assume that the page is accessible by user space but not globally accessible.
    pub fn new() -> Self
    {
        Permissions
            {
                readable:            true,
                writable:            false,
                executable:          false,

                user_accessible:     true,

                globally_accessible: false
            }
    }

    /// Create a new `PermissionsBuilder` to build a `Permissions` object with custom values.
    pub fn builder() -> PermissionsBuilder
    {
        PermissionsBuilder::default()
    }
}



impl Default for Permissions
{
    /// Create a new `Permissions` object with default values. This will create a set of permissions
    /// that allows the page to be read, but not written to or executed. But also user accessible.
    fn default() -> Self
    {
        Self::new()
    }
}



impl Display for Permissions
{
    /// Format the permissions as a string for debugging or other display purposes.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        write!(f,
               "{} - {} - <{}{}{}>",
               if self.globally_accessible { "globally" } else { "locally" },
               if self.user_accessible     { "user"     } else { "kernel"  },
               if self.readable            { "r"        } else { "-"       },
               if self.writable            { "w"        } else { "-"       },
               if self.executable          { "x"        } else { "-"       })
    }
}
