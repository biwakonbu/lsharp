pub(crate) const DRIVER_IO_ERROR_CODE: &str = "LS5001";

pub(crate) fn driver_io_error(message: impl std::fmt::Display) -> miette::Report {
    miette::miette!("[{DRIVER_IO_ERROR_CODE}] {message}")
}
