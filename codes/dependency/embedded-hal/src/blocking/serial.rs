//! Blocking serial API

/// Write half of a serial interface (blocking variant)
pub trait Write<Word> {
    /// The type of error that can occur when writing
    type Error;

    /// Writes a slice, blocking until everything has been written
    ///
    /// An implementation can choose to buffer the write, returning `Ok(())`
    /// after the complete slice has been written to a buffer, but before all
    /// words have been sent via the serial interface. To make sure that
    /// everything has been sent, call [`try_bflush`] after this function returns.
    ///
    /// [`try_bflush`]: #tymethod.bflush
    fn try_bwrite_all(&mut self, buffer: &[Word]) -> Result<(), Self::Error>;

    /// Block until the serial interface has sent all buffered words
    fn try_bflush(&mut self) -> Result<(), Self::Error>;
}

/// Blocking serial write
pub mod write {
    /// Marker trait to opt into default blocking write implementation
    ///
    /// Implementers of [`serial::Write`] can implement this marker trait
    /// for their type. Doing so will automatically provide the default
    /// implementation of [`blocking::serial::Write`] for the type.
    ///
    /// [`serial::Write`]: ../../serial/trait.Write.html
    /// [`blocking::serial::Write`]: ../trait.Write.html
    pub trait Default<Word>: crate::serial::Write<Word> {}

    impl<S, Word> crate::blocking::serial::Write<Word> for S
    where
        S: Default<Word>,
        Word: Clone,
    {
        type Error = S::Error;

        fn try_bwrite_all(&mut self, buffer: &[Word]) -> Result<(), Self::Error> {
            for word in buffer {
                block!(self.try_write(word.clone()))?;
            }

            Ok(())
        }

        fn try_bflush(&mut self) -> Result<(), Self::Error> {
            block!(self.try_flush())?;
            Ok(())
        }
    }
}
