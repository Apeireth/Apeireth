//! Delegation over the existing native Windows child implementations.
use super::*;

impl ManagedChild for WindowsChild {
    fn try_wait(&mut self) -> Result<Option<ChildStatus>, ProcessError> {
        match self {
            Self::Std(child) => child.try_wait(),
            Self::Raw(child) => child.try_wait(),
        }
    }

    fn wait(&mut self) -> Result<ChildStatus, ProcessError> {
        match self {
            Self::Std(child) => child.wait(),
            Self::Raw(child) => child.wait(),
        }
    }

    fn wait_timeout(&mut self, timeout: Duration) -> Result<Option<ChildStatus>, ProcessError> {
        match self {
            Self::Std(child) => child.wait_timeout(timeout),
            Self::Raw(child) => child.wait_timeout(timeout),
        }
    }

    fn terminate(&mut self) -> Result<(), ProcessError> {
        match self {
            Self::Std(child) => child.terminate(),
            Self::Raw(child) => child.terminate(),
        }
    }

    fn terminate_remaining(&mut self) -> Result<(), ProcessError> {
        match self {
            Self::Std(child) => child.terminate_remaining(),
            Self::Raw(child) => child.terminate_remaining(),
        }
    }

    fn take_stdout(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        match self {
            Self::Std(child) => child.take_stdout(),
            Self::Raw(child) => child.take_stdout(),
        }
    }

    fn take_stderr(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        match self {
            Self::Std(child) => child.take_stderr(),
            Self::Raw(child) => child.take_stderr(),
        }
    }
}
