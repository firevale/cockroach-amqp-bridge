use smol_str::SmolStr;

#[derive(Debug, PartialEq)]
pub enum Envelope {
  Row,
  KeyOnly,
}

impl Default for Envelope {
  fn default() -> Self {
    Self::Row
  }
}

#[derive(Debug, PartialEq, Default)]
pub struct ChangefeedOptions {
  pub envelope: Envelope,
  pub cursor: Option<SmolStr>,
  pub table_name: SmolStr,
}

impl ChangefeedOptions {
  pub fn query_string(&self) -> String {
    if self.table_name.is_empty() {
      panic!("can not capture change feed without table name");
    }

    let mut query: String = format!(
      "EXPERIMENTAL CHANGEFEED FOR {} with updated, resolved = '10s'",
      self.table_name
    );

    if self.envelope == Envelope::KeyOnly {
      query.push_str(r#",envelope=key_only"#);
    }

    if let Some(cursor) = self.cursor.as_deref() {
      query.push_str(r#", cursor='"#);
      query.push_str(&cursor);
      query.push_str(r#"'"#);
    }

    query.push_str(r#";"#);
    query
  }
}
