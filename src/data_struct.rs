use crate::list::{List, VariableList, SetOutcome, ListError, VariableListError};
use std::collections::BTreeMap;

pub struct DataStruct<'a, Character> {
  schema: &'a Character,
  index:  List<usize>,
  values: VariableList<u8>,
}

pub trait CharacterField {
    fn id(&self) -> usize;
}

impl<'a, C: CharacterField> DataStruct<'a, C> {
  const INDEX_WIDTH: usize = 1;

  pub fn get(&self, field: &C) -> Result<&[u8], ListError> {
      self.index.get(&field.id(), &Self::INDEX_WIDTH)?;
      self.values.get(&field.id())
  }

  pub fn set(&mut self, field: &C, value: &[u8]) -> Result<SetOutcome, ListError> {
      self.index.set(&field.id(), &Self::INDEX_WIDTH, &[field.id()], false)?;
      self.values.set(&field.id(), value, false)
  }

  pub fn delete(&mut self, field: &C) -> Result<(), ListError> {
      self.index.delete(&field.id(), &mut { Self::INDEX_WIDTH })?;
      self.values.delete(&field.id())
  }

  pub fn compact(&mut self) -> Result<BTreeMap<usize, usize>, VariableListError> {
      self.values.compact()
  }
}