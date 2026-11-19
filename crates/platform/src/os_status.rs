//! Код ошибки 0 в Windows означает «Операция успешно завершена». Если системный вызов вернул
//! признак неудачи, а последний код ошибки при этом 0 (так бывает у `SystemParametersInfoW` и
//! у чтения/записи реестра), это не ошибка: оформление записано, и настоящий успех проверяется
//! чтением значений обратно. Здесь чистые функции без системных вызовов, они проверяются на
//! любой ОС.

use std::io;

/// Код 0 (в виде `Win32` или `HRESULT`) — успех, а не ошибка.
pub fn is_success_code(code: i32) -> bool {
    code == 0
}

/// Результат ввода-вывода: ошибка с кодом ОС 0 считается успехом.
pub fn io_ok_if_code_zero(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(e) if e.raw_os_error().is_some_and(is_success_code) => Ok(()),
        other => other,
    }
}

/// Результат системного вызова с кодом `code` и описанием `text`: код 0 — успех.
pub fn code_result(code: i32, text: impl FnOnce() -> String) -> Result<(), String> {
    if is_success_code(code) {
        Ok(())
    } else {
        Err(text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_with_os_code_zero_is_success() {
        assert!(io_ok_if_code_zero(Err(io::Error::from_raw_os_error(0))).is_ok());
        assert!(io_ok_if_code_zero(Ok(())).is_ok());
    }

    #[test]
    fn a_real_os_error_stays_an_error() {
        let result = io_ok_if_code_zero(Err(io::Error::from_raw_os_error(5)));
        assert_eq!(result.unwrap_err().raw_os_error(), Some(5));
        assert!(io_ok_if_code_zero(Err(io::Error::other("нет кода"))).is_err());
    }

    #[test]
    fn windows_code_zero_is_success() {
        assert_eq!(
            code_result(0, || "Операция успешно завершена".into()),
            Ok(())
        );
        assert_eq!(code_result(5, || "отказано".into()), Err("отказано".into()));
    }
}
