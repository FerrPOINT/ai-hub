#[derive(Debug, Clone, thiserror::Error)]
pub enum HubError {
    #[error("Требуется вход в платформу")]
    Unauthenticated,
    #[error("Недостаточно прав")]
    Forbidden,
    #[error("Объект не найден")]
    NotFound,
    #[error("Конфигурация изменилась; сохраните черновик и обновите версию")]
    PreconditionFailed,
    #[error("Ключ операции уже связан с другим запросом")]
    IdempotencyConflict,
    #[error("Доступный бюджет или лимит запросов исчерпан")]
    BudgetExceeded,
    #[error("Некорректный запрос: {0}")]
    Invalid(&'static str),
    #[error("Сервис временно недоступен")]
    Unavailable,
    #[error("Не подтверждена идентичность установки или схема")]
    InstallationMismatch,
}
