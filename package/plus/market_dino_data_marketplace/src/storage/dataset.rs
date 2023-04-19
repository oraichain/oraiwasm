use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};

use crate::model::dataset::NormalDataset;

pub struct NormalDatasetIndexes<'a> {
    pub token_id: MultiIndex<'a, NormalDataset>,
    pub owner_addr: MultiIndex<'a, NormalDataset>,
    pub datasource: MultiIndex<'a, NormalDataset>,
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
        token_id: MultiIndex::new(
            |o| o.token_id.as_bytes().to_vec(),
            "normal_dataset",
            "normal_dataset__token_id",
        ),
        owner_addr: MultiIndex::new(
            |o| o.owner.as_bytes().to_vec(),
            "normal_dataset",
            "normal_dataset__owner_addr",
        ),
        datasource: MultiIndex::new(
            |o| o.datasource.get_name().as_bytes().to_vec(),
            "normal_dataset",
            "normal_dataset_datasource",
        ),
    };
    IndexedMap::new("normal_dataset", indexes)
}
