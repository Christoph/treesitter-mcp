#[cfg(test)]
mod tests {
    use crate::extraction::types::*;
    use eyre::Result;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_rust_extraction() -> Result<()> {
        let source = r#"
            pub struct User {
                pub id: u64,
                pub name: String,
            }
            enum Status { Active, Inactive }
            trait Auth { type Item; fn login(&self) -> Self::Item; }
            type UserId = u64;
        "#;
        let dir = tempdir()?;
        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, source)?;

        let result = extract_rust_types(source, Path::new("test.rs"))?;
        assert_eq!(result.len(), 4);

        let user = result.iter().find(|t| t.name == "User").unwrap();
        assert_eq!(user.kind, TypeKind::Struct);
        assert_eq!(user.fields.as_ref().unwrap().len(), 2);

        let status = result.iter().find(|t| t.name == "Status").unwrap();
        assert_eq!(status.kind, TypeKind::Enum);
        assert_eq!(status.variants.as_ref().unwrap().len(), 2);

        let auth = result.iter().find(|t| t.name == "Auth").unwrap();
        assert_eq!(auth.kind, TypeKind::Trait);
        let members = auth.members.as_ref().unwrap();
        assert!(members.iter().any(|m| m.name == "Item"));
        assert!(members.iter().any(|m| m.name == "login"));

        Ok(())
    }

    #[test]
    fn test_typescript_extraction() -> Result<()> {
        let source = r#"
            export class UserService {
                private db: Database;
                constructor(db: Database) { this.db = db; }
                async getUser(id: number): Promise<User> { return null; }
            }
            interface User {
                id: number;
                name: string;
            }
            enum Role { Admin, User }
            type Id = number;
        "#;
        let result = extract_typescript_types(source, Path::new("test.ts"), true)?;
        assert_eq!(result.len(), 4);

        let service = result.iter().find(|t| t.name == "UserService").unwrap();
        assert_eq!(service.kind, TypeKind::Class);

        let user = result.iter().find(|t| t.name == "User").unwrap();
        assert_eq!(user.kind, TypeKind::Interface);
        assert_eq!(user.members.as_ref().unwrap().len(), 2);

        Ok(())
    }

    #[test]
    fn test_python_extraction() -> Result<()> {
        let source = r#"
class User:
    id: int
    def __init__(self, name: str):
        self.name = name

class Status(Enum):
    ACTIVE = 1
    INACTIVE = 2

class Reader(Protocol):
    name: str
    def read(self) -> str: ...

UserDict = TypedDict("UserDict", {"id": int, "name": str})
Point = NamedTuple("Point", [("x", int), ("y", int)])
"#;

        let result = extract_python_types(source, Path::new("test.py"))?;
        assert_eq!(result.len(), 5);

        let user = result.iter().find(|t| t.name == "User").unwrap();
        assert_eq!(user.kind, TypeKind::Class);
        // Should find both id (type hint) and name (self assignment)
        assert_eq!(user.fields.as_ref().unwrap().len(), 2);

        let status = result.iter().find(|t| t.name == "Status").unwrap();
        assert_eq!(status.kind, TypeKind::Enum);
        assert_eq!(status.variants.as_ref().unwrap().len(), 2);

        let reader = result.iter().find(|t| t.name == "Reader").unwrap();
        assert_eq!(reader.kind, TypeKind::Protocol);
        let members = reader.members.as_ref().unwrap();
        assert!(members.iter().any(|m| m.name == "read"));
        assert!(members.iter().any(|m| m.name == "name"));

        let user_dict = result.iter().find(|t| t.name == "UserDict").unwrap();
        assert_eq!(user_dict.kind, TypeKind::TypedDict);
        assert_eq!(user_dict.fields.as_ref().unwrap().len(), 2);

        let point = result.iter().find(|t| t.name == "Point").unwrap();
        assert_eq!(point.kind, TypeKind::NamedTuple);
        assert_eq!(point.fields.as_ref().unwrap().len(), 2);

        Ok(())
    }
}
