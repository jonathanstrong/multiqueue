use countedindex::Index;
use multiqueue::{InnerSend, InnerRecv, BCast, MPMC, MultiQueue};
use wait::Wait;

use std::sync::mpsc::{TrySendError, TryRecvError, RecvError};

/// This class is the sending half of the MultiQueue. It supports both
/// single and multi consumer modes with competitive performance in each case.
/// It only supports nonblocking writes (the futures sender being an exception)
/// as well as being the conduit for adding new writers.
///
/// # Examples
///
/// ```
/// use std::thread;
///
/// let (send, recv) = multiqueue::multiqueue(4);
///
/// let mut handles = vec![];
///
/// for i in 0..2 { // or n
///     let consumer = recv.clone();
///     handles.push(thread::spawn(move || {
///         for val in consumer {
///             println!("Consumer {} got {}", i, val);
///         }
///     }));
/// }
///
/// // Take notice that I drop the reader - this removes it from
/// // the queue, meaning that the readers in the new threads
/// // won't get starved by the lack of progress from recv
/// recv.unsubscribe();
///
/// for i in 0..10 {
///     // Don't do this busy loop in real stuff unless you're really sure
///     loop {
///         if send.try_send(i).is_ok() {
///             break;
///         }
///     }
/// }
/// drop(send);
///
/// for t in handles {
///     t.join();
/// }
/// // prints along the lines of
/// // Consumer 1 got 2
/// // Consumer 0 got 0
/// // Consumer 0 got 1
/// // etc
/// ```
#[derive(Clone)]
pub struct MPMCSender<T> {
    sender: InnerSend<MPMC<T>, T>,
}


/// This is the receiving end of a standard mpmc view of the queue
/// It functions similarly to the broadcast queue execpt there
/// is only ever one stream. As a result, the type doesn't need to be clone
#[derive(Clone)]
pub struct MPMCReceiver<T> {
    reader: InnerRecv<MPMC<T>, T>,
}


/// This is the receiving end of a standard mpmc view of the queue
/// for when it's statically know that there is only one receiver.
/// It functions similarly to the broadcast queue UniReceiver execpt there
/// is only ever one stream. As a result, the type doesn't need to be clone or sync
pub struct MPMCUniReceiver<T> {
    reader: InnerRecv<MPMC<T>, T>,
}


impl<T> MPMCSender<T> {
    /// Tries to send a value into the queue
    /// If there is no space, returns Err(TrySendError::Full(val))
    /// If there are no readers, returns Err(TrySendError::Disconnected(val))
    pub fn try_send(&self, val: T) -> Result<(), TrySendError<T>> {
        self.sender.try_send(val)
    }

    /// Removes this writer from the queue
    pub fn unsubscribe(self) {
        self.sender.unsubscribe()
    }
}

impl<T> MPMCReceiver<T> {
    /// Tries to receive a value from the queue without blocking.
    ///
    /// # Examples:
    ///
    /// ```
    /// use multiqueue::mpmc_queue;
    /// let (w, r) = mpmc_queue(10);
    /// w.try_send(1).unwrap();
    /// assert_eq!(1, r.try_recv().unwrap());
    /// ```
    ///
    /// ```
    /// use multiqueue::multiqueue;
    /// use std::thread;
    ///
    /// let (send, recv) = multiqueue(10);
    ///
    /// let handle = thread::spawn(move || {
    ///     for val in recv {
    ///         println!("Got {}", val);
    ///     }
    /// });
    ///
    /// for i in 0..10 {
    ///     send.try_send(i).unwrap();
    /// }
    ///
    /// // Drop the sender to close the queue
    /// drop(send);
    ///
    /// handle.join();
    /// ```

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.reader.try_recv()
    }

    pub fn recv(&self) -> Result<T, RecvError> {
        self.reader.recv()
    }

    /// Removes the given reader from the queue subscription lib
    /// Returns true if this is the last reader in a given broadcast unit
    ///
    /// # Examples
    ///
    /// ```
    /// use multiqueue::multiqueue;
    /// let (writer, reader) = multiqueue(2);
    /// writer.try_send(1).expect("This will succeed since queue is empty");
    /// reader.try_recv().expect("This reader can read");
    /// reader.unsubscribe();
    /// // Fails since there's no readers left
    /// assert!(writer.try_send(1).is_err());
    /// ```
    pub fn unsubscribe(self) -> bool {
        self.reader.unsubscribe()
    }
}

/*
/// If there is only one InnerRecv on the stream, converts the
/// InnerRecv into a UniInnerRecv otherwise returns the InnerRecv.
///
/// # Example:
///
/// ```
/// use multiqueue::multiqueue;
///
/// let (w, r) = multiqueue(10);
/// w.try_send(1).unwrap();
/// let r2 = r.clone();
/// // Fails since there's two receivers on the stream
/// assert!(r2.into_single().is_err());
/// let single_r = r.into_single().unwrap();
/// let val = match single_r.try_recv_view(|x| 2 * *x) {
///     Ok(val) => val,
///     Err(_) => panic!("Queue should have an element"),
/// };
/// assert_eq!(2, val);
//    pub fn into_single(&self) -> Result<Receiver<T>, Sender<T>> {
//
//   }
 */

impl<T> MPMCUniReceiver<T> {
    /// Identical to MPMCReceiver::try_recv
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.reader.try_recv()
    }

    /// Identical to MPMCReceiver::recv
    pub fn recv(&self) -> Result<T, RecvError> {
        self.reader.recv()
    }


    /// Similar to UniMcastReceiver::try_recv_view, except this closure takes
    pub fn try_recv_view<R, F: FnOnce(&T) -> R>(&self, op: F) -> Result<R, (F, TryRecvError)> {
        self.reader.try_recv_view(op)
    }

    /// Similar to UniMcastReceiver::recv_view
    pub fn recv_view<R, F: FnOnce(&T) -> R>(&self, op: F) -> Result<R, (F, RecvError)> {
        self.reader.recv_view(op)
    }

    /// Removes the given reader from the queue subscription lib
    /// Returns true if this is the last reader in a given broadcast unit
    ///
    /// # Examples
    ///
    /// ```
    /// use multiqueue::multiqueue;
    /// let (writer, reader) = multiqueue(2);
    /// writer.try_send(1).expect("This will succeed since queue is empty");
    /// reader.try_recv().expect("This reader can read");
    /// reader.unsubscribe();
    /// // Fails since there's no readers left
    /// assert!(writer.try_send(1).is_err());
    /// ```
    pub fn unsubscribe(self) -> bool {
        self.reader.unsubscribe()
    }
}

pub struct MPMCIter<T> {
    recv: MPMCReceiver<T>,
}

impl<T> Iterator for MPMCIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self.recv.recv() {
            Ok(val) => Some(val),
            Err(_) => None,
        }
    }
}

impl<T> IntoIterator for MPMCReceiver<T> {
    type Item = T;

    type IntoIter = MPMCIter<T>;

    fn into_iter(self) -> MPMCIter<T> {
        MPMCIter { recv: self }
    }
}


pub fn mpmc_queue<T>(capacity: Index) -> (MPMCSender<T>, MPMCReceiver<T>) {
    let (send, recv) = MultiQueue::<MPMC<T>, T>::new(capacity);
    (MPMCSender { sender: send }, MPMCReceiver { reader: recv })
}

pub fn mpmc_queue_with<T, W: Wait + 'static>(capacity: Index,
                                             w: W)
                                             -> (MPMCSender<T>, MPMCReceiver<T>) {
    let (send, recv) = MultiQueue::<MPMC<T>, T>::new_with(capacity, w);
    (MPMCSender { sender: send }, MPMCReceiver { reader: recv })
}

unsafe impl<T: Send> Send for MPMCSender<T> {}
unsafe impl<T: Send> Send for MPMCReceiver<T> {}
unsafe impl<T: Send> Send for MPMCUniReceiver<T> {}
