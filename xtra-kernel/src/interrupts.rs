

pub type InterruptHandler = fn() -> ();



pub fn initialize_interrupts() -> Result<(), &'static str>
{
     Ok(())
}


pub fn enable_interrupts(core_id: Option<usize>)
{
}


pub fn disable_interrupts(core_id: Option<usize>)
{
}


pub fn register_interrupt_handler(core_id: Option<usize>,
                                  interrupt_number: usize,
                                  handler: InterruptHandler)
{
}


pub fn remove_interrupt_handler(core_id: Option<usize>, interrupt_number: usize)
{
}


pub fn pause_interrupt_handler(core_id: Option<usize>, interrupt_number: usize)
{
}
