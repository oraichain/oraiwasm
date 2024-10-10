use cosmwasm_std::{DepsMut, StdResult};
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex, UniqueIndex};

use crate::model::dataset::{DatasetFactory, NormalDataset};

pub struct NormalDatasetIndexes<'a> {
    pub token_id: UniqueIndex<'a, Vec<u8>, NormalDataset>,
    pub owner_addr: MultiIndex<'a, Vec<u8>, NormalDataset, &'a [u8]>,
    pub datasource: MultiIndex<'a, Vec<u8>, NormalDataset, &'a [u8]>,
    pub d_type: MultiIndex<'a, Vec<u8>, NormalDataset, &'a [u8]>,
}

impl<'a> IndexList<NormalDataset> for NormalDatasetIndexes<'a> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<NormalDataset>> + '_> {
        let v: Vec<&dyn Index<NormalDataset>> =
            vec![&self.token_id, &self.owner_addr, &self.datasource];
        Box::new(v.into_iter())
    }
}

pub fn storage_datasets<'a>() -> IndexedMap<'a, &'a [u8], NormalDataset, NormalDatasetIndexes<'a>> {
    let indexes = NormalDatasetIndexes {
        owner_addr: MultiIndex::new(
            |_pk, o| o.owner.as_bytes().to_vec(),
            "normal_dataset",
            "normal_dataset__owner_addr",
        ),
        datasource: MultiIndex::new(
            |_pk, o| o.datasource.get_name().as_bytes().to_vec(),
            "normal_dataset",
            "normal_dataset__datasource",
        ),
        d_type: MultiIndex::new(
            |_pk, o| o.to_owned().get_type().as_bytes().to_vec(),
            "normal_dataset",
            "normal_dataset__type",
        ),
        token_id: UniqueIndex::new(|o| o.token_id.as_bytes().to_vec(), "normal_dataset"),
    };
    IndexedMap::new("normal_dataset", indexes)
}

pub fn get_normal_dataset_by_id(deps: DepsMut, token_id: &str) -> StdResult<NormalDataset> {
    storage_datasets().load(deps.storage, &token_id.as_bytes())
}
