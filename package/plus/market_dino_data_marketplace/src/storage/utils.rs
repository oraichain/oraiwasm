pub trait StorageMapper<DTO> {
    fn to_db(&self) -> DTO;
}
