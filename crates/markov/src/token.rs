use crate::Word;

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Token {
    Word(Word), // TODO a thin box
    End,
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut token = f.debug_struct("Token");
        match self {
            Self::Word(s) => token.field("kind", &"Word").field("len", &s.len()),
            Self::End => token.field("end", &"End"),
        }
        .finish()
    }
}
