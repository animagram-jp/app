pub struct DataStruct<'a> {
  schema: &'a Character,
  index:  List<usize>,
  values: VariableList<u8>,
}

impl<'a> DataStruct<'a> {
  const INDEX_WIDTH: usize = 1;

  pub fn get(&self, field: &Character) -> Result<&[u8], ListError> {
      self.id.get(&field.id(), &Self::INDEX_WIDTH)?;
      self.values.get(&field.id())
  }

  pub fn set(&mut self, field: &Character, value: &[u8]) -> Result<SetOutcome, ListError> {
      self.id.set(&field.id(), &Self::INDEX_WIDTH, &[field.id()], false)?;
      self.values.set(&field.id(), value, false)
  }

  pub fn delete(&mut self, field: &Character) -> Result<(), ListError> {
      self.id.delete(&field.id(), &Self::INDEX_WIDTH)?;
      self.values.delete(&field.id())
  }

  pub fn compact(&mut self) -> Result<BTreeMap<usize, usize>, VariableListError> {
      self.values.compact()
  }

  pub fn update(&mut self, field: &Character) -> Result<(), ListError> {
      match field {
          Character::Characteristic(_) => {
              for d in Derived::all() {
                  let v = d.compute(self)?;
                  self.set(&Character::Derived(d), &v)?;
              }
              for s in Skill::all() {
                  let v = s.compute(self)?;
                  self.set(&Character::Skill(s), &v)?;
              }
          }
          _ => {}
      }
      Ok(())
  }
}