use std::collections::HashMap;
use std::collections::HashSet;
use heck::{ToUpperCamelCase, ToShoutySnakeCase};

struct SymbolTable {
    types: HashMap<String, String>,
    functions: HashMap<String, String>,
    variables: HashMap<String, String>,
    constants: HashMap<String, String>,
    arrays: HashSet<String>,
}

impl SymbolTable {
    fn register_type(&mut self, c_name: &str) {
        if !self.types.contains_key(c_name) {
            self.types.insert(c_name.to_string(), to_c3_type_name(c_name));
        }
    }

    fn register_function(&mut self, c_name: &str) {
        if !self.functions.contains_key(c_name) {
            self.functions.insert(c_name.to_string(), to_c3_variable_name(c_name));
        }
    }

    fn register_variable(&mut self, c_name: &str) {
        if !self.variables.contains_key(c_name) {
            self.variables.insert(c_name.to_string(), to_c3_variable_name(c_name));
        }
    }

    fn register_constant(&mut self, c_name: &str) {
        if !self.constants.contains_key(c_name) {
            self.constants.insert(c_name.to_string(), c_name.to_shouty_snake_case());
        }
    }
}

fn to_c3_type_name(c_name: &str) -> String {
    if c_name.is_empty() { return String::new(); }
    
    let mut leading_underscores = String::new();
    let mut rest = c_name;
    
    for c in c_name.chars() {
        if c == '_' {
            leading_underscores.push('_');
            rest = &rest[1..];
        } else {
            break;
        }
    }
    
    if rest.is_empty() {
        return "T".to_string();
    }
    
    let mut camel = rest.to_upper_camel_case();
    
    if camel.chars().all(|c| !c.is_lowercase()) {
        let mut chars = camel.chars();
        if let Some(first) = chars.next() {
            let mut new_camel = first.to_string();
            new_camel.push_str(&chars.as_str().to_lowercase());
            camel = new_camel;
        }
    }
    
    format!("{}{}", leading_underscores, camel)
}

fn to_c3_variable_name(c_name: &str) -> String {
    if c_name.is_empty() { return String::new(); }
    
    let mut leading_underscores = String::new();
    let mut rest = c_name;
    
    for c in c_name.chars() {
        if c == '_' {
            leading_underscores.push('_');
            rest = &rest[1..];
        } else {
            break;
        }
    }
    
    if rest.is_empty() {
        return c_name.to_string();
    }
    
    let mut chars = rest.chars();
    if let Some(first) = chars.next() {
        if first.is_uppercase() {
            let mut new_name = leading_underscores;
            new_name.push_str(&first.to_lowercase().to_string());
            new_name.push_str(chars.as_str());
            new_name
        } else {
            c_name.to_string()
        }
    } else {
        c_name.to_string()
    }
}

fn extract_declarator_identifier(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" => Some(node),
        "init_declarator" => {
            if let Some(decl) = node.child_by_field_name("declarator") {
                extract_declarator_identifier(decl)
            } else {
                None
            }
        }
        "pointer_declarator" | "array_declarator" | "parenthesized_declarator" => {
            if let Some(decl) = node.child_by_field_name("declarator") {
                extract_declarator_identifier(decl)
            } else if node.child_count() > 0 {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if let Some(id) = extract_declarator_identifier(child) {
                        return Some(id);
                    }
                }
                None
            } else {
                None
            }
        }
        _ => {
            if node.child_count() > 0 {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if let Some(id) = extract_declarator_identifier(child) {
                        return Some(id);
                    }
                }
            }
            None
        }
    }
}

fn find_array_declarator(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    if node.kind() == "array_declarator" {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(arr) = find_array_declarator(child) {
            return Some(arr);
        }
    }
    None
}

fn get_array_dimensions<'a>(mut node: tree_sitter::Node<'a>, source: &'a [u8], symbols: &'a SymbolTable) -> (Vec<String>, Option<tree_sitter::Node<'a>>) {
    let mut dims = Vec::new();
    while node.kind() == "array_declarator" {
        if let Some(size_node) = node.child_by_field_name("size") {
            dims.push(transpile_node(size_node, source, symbols));
        } else {
            dims.push(String::new());
        }
        if let Some(inner) = node.child_by_field_name("declarator") {
            node = inner;
        } else {
            break;
        }
    }
    dims.reverse();
    (dims, Some(node))
}

fn is_pointer_declarator(node: tree_sitter::Node) -> bool {
    if node.kind() == "pointer_declarator" {
        return true;
    }
    if let Some(inner) = node.child_by_field_name("declarator") {
        is_pointer_declarator(inner)
    } else {
        false
    }
}

fn get_raw_identifier_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    if node.kind() == "identifier" {
        if let Ok(text) = node.utf8_text(source) {
            return Some(text.to_string());
        }
    }
    None
}

fn has_return_statement(node: tree_sitter::Node) -> bool {
    if node.kind() == "return_statement" {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_return_statement(child) {
            return true;
        }
    }
    false
}

fn convert_c_type(c_type: &str) -> Option<String> {
    match c_type {
        "int8_t" | "signed char" => Some("ichar".to_string()),
        "uint8_t" | "unsigned char" => Some("char".to_string()),
        "int16_t" | "short" | "short int" => Some("short".to_string()),
        "uint16_t" | "unsigned short" | "unsigned short int" => Some("ushort".to_string()),
        "int32_t" | "int" => Some("int".to_string()),
        "uint32_t" | "unsigned" | "unsigned int" => Some("uint".to_string()),
        "int64_t" | "long" | "long int" | "long long" | "long long int" => Some("long".to_string()),
        "uint64_t" | "unsigned long" | "unsigned long int" | "unsigned long long" | "unsigned long long int" => Some("ulong".to_string()),
        "int128_t" | "__int128" => Some("int128".to_string()),
        "uint128_t" | "__uint128_t" | "unsigned __int128" => Some("uint128".to_string()),
        "size_t" => Some("usz".to_string()),
        "ssize_t" => Some("sz".to_string()),
        "ptrdiff_t" => Some("sz".to_string()),
        "intptr_t" => Some("iptr".to_string()),
        "uintptr_t" => Some("uptr".to_string()),
        _ => None,
    }
}

fn transpile_const_enum(node: tree_sitter::Node, source: &[u8], symbols: &SymbolTable) -> Option<String> {
    let text = node.utf8_text(source).unwrap_or("");
    if text.starts_with("const") && text.contains('{') && !text.contains('=') {
        let before_brace = text.split('{').next()?;
        let parts: Vec<&str> = before_brace.split_whitespace().collect();
        if parts.len() >= 2 && parts[0] == "const" {
            let name = parts[1];
            let c3_name = symbols.types.get(name).unwrap_or(&name.to_string()).clone();
            
            let braced_part = text.split_once('{')?.1;
            let body_content = braced_part.split_once('}')?.0;
            
            let mut elements = Vec::new();
            for item in body_content.split(',') {
                let trimmed = item.trim();
                if !trimmed.is_empty() {
                    elements.push(trimmed.to_string());
                }
            }
            
            let has_gap = elements.iter().any(|e| e.contains('='));
            
            let mut body_out = String::from("{\n");
            for (idx, elem) in elements.iter().enumerate() {
                body_out.push_str("    ");
                body_out.push_str(elem);
                if idx < elements.len() - 1 {
                    body_out.push_str(",\n");
                } else {
                    body_out.push('\n');
                }
            }
            body_out.push('}');
            
            if has_gap {
                return Some(format!("constdef {} {}", c3_name, body_out));
            } else {
                return Some(format!("enum {} {}", c3_name, body_out));
            }
        }
    }
    None
}

fn transpile_switch_case_statements(statements: &[tree_sitter::Node], source: &[u8], symbols: &SymbolTable) -> (String, bool) {
    let mut real_statements = Vec::new();
    for &child in statements {
        if child.kind() != "comment" {
            real_statements.push(child);
        }
    }
    
    let mut out = String::new();
    let mut skip_last_break = false;
    let mut need_nextcase = false;
    
    if let Some(&last) = real_statements.last() {
        if last.kind() == "break_statement" {
            skip_last_break = true;
        } else if last.kind() != "return_statement" && last.kind() != "continue_statement" && last.kind() != "goto_statement" {
            need_nextcase = true;
        }
    }
    
    for &child in statements {
        if skip_last_break {
            if let Some(last_node) = real_statements.last() {
                if child.id() == last_node.id() {
                    continue;
                }
            }
        }
        
        let child_str = transpile_node(child, source, symbols);
        if !child_str.trim().is_empty() {
            out.push_str("    ");
            out.push_str(&child_str.replace("\n", "\n    "));
            out.push('\n');
        }
    }
    
    if need_nextcase {
        out.push_str("    nextcase;\n");
    }
    
    (out.trim_end().to_string(), need_nextcase)
}

fn transpile_node(node: tree_sitter::Node, source: &[u8], symbols: &SymbolTable) -> String {
    let kind = node.kind();
    
    match kind {
        "translation_unit" => {
            let mut out = String::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == ";" {
                    continue;
                }
                
                let child_output = transpile_node(child, source, symbols);
                if child_output.trim().is_empty() {
                    continue;
                }
                
                out.push_str(&child_output);
                out.push('\n');
            }
            out
        }

        "struct_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let c_name = name_node.utf8_text(source).unwrap_or("");
                let c3_name = symbols.types.get(c_name).unwrap_or(&c_name.to_string()).clone();
                
                if let Some(body_node) = node.child_by_field_name("body") {
                    let c3_body = transpile_node(body_node, source, symbols);
                    format!("struct {} @cname(\"{}\") {}", c3_name, c_name, c3_body)
                } else {
                    c3_name
                }
            } else {
                if let Some(body_node) = node.child_by_field_name("body") {
                    let c3_body = transpile_node(body_node, source, symbols);
                    format!("struct {}", c3_body)
                } else {
                    "struct".to_string()
                }
            }
        }

        "enum_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let c_name = name_node.utf8_text(source).unwrap_or("");
                let c3_name = symbols.types.get(c_name).unwrap_or(&c_name.to_string()).clone();
                
                if let Some(body_node) = node.child_by_field_name("body") {
                    let has_gap = body_node.utf8_text(source).unwrap_or("").contains('=');
                    
                    let mut body_out = String::from("{\n");
                    let mut cursor = body_node.walk();
                    let mut elements = Vec::new();
                    for child in body_node.children(&mut cursor) {
                        if child.kind() == "enumerator" {
                            elements.push(transpile_node(child, source, symbols));
                        }
                    }
                    
                    for (idx, elem) in elements.iter().enumerate() {
                        body_out.push_str("    ");
                        body_out.push_str(elem);
                        if idx < elements.len() - 1 {
                            body_out.push_str(",\n");
                        } else {
                            body_out.push('\n');
                        }
                    }
                    body_out.push('}');
                    
                    if has_gap {
                        format!("constdef {} {}", c3_name, body_out)
                    } else {
                        format!("enum {} {}", c3_name, body_out)
                    }
                } else {
                    c3_name
                }
            } else {
                "enum".to_string()
            }
        }

        "union_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let c_name = name_node.utf8_text(source).unwrap_or("");
                let c3_name = symbols.types.get(c_name).unwrap_or(&c_name.to_string()).clone();
                
                if let Some(body_node) = node.child_by_field_name("body") {
                    let c3_body = transpile_node(body_node, source, symbols);
                    format!("union {} {}", c3_name, c3_body)
                } else {
                    c3_name
                }
            } else {
                if let Some(body_node) = node.child_by_field_name("body") {
                    let c3_body = transpile_node(body_node, source, symbols);
                    format!("union {}", c3_body)
                } else {
                    "union".to_string()
                }
            }
        }

        "enumerator" => {
            if let Some(name) = node.child_by_field_name("name") {
                let name_str = transpile_node(name, source, symbols);
                if let Some(value) = node.child_by_field_name("value") {
                    let value_str = transpile_node(value, source, symbols);
                    format!("{} = {}", name_str, value_str)
                } else {
                    name_str
                }
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "field_declaration_list" => {
            let mut out = String::from("{\n");
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                let child_kind = child.kind();
                if child_kind == "{" || child_kind == "}" {
                    continue; 
                }
                out.push_str("    ");
                out.push_str(&transpile_node(child, source, symbols));
                out.push('\n');
            }
            out.push('}');
            out
        }

        "function_definition" => {
            let type_node = node.child_by_field_name("type");
            let decl_node = node.child_by_field_name("declarator");
            let body_node = node.child_by_field_name("body");
            
            if let (Some(t), Some(d), Some(b)) = (type_node, decl_node, body_node) {
                let ret_type = transpile_node(t, source, symbols);
                let c3_decl = transpile_node(d, source, symbols);
                let mut c3_body = transpile_node(b, source, symbols);
                
                let is_main = if let Some(id_node) = extract_declarator_identifier(d) {
                    id_node.utf8_text(source).unwrap_or("") == "main"
                } else {
                    false
                };

                if is_main && ret_type == "int" && !has_return_statement(b) {
                    if c3_body.ends_with('}') {
                        c3_body.pop();
                        if !c3_body.ends_with('\n') {
                            c3_body.push('\n');
                        }
                        c3_body.push_str("    return 0;\n}");
                    }
                }

                format!("fn {} {} {}", ret_type, c3_decl, c3_body)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "function_declarator" => {
            if let (Some(declarator), Some(parameters)) = (node.child_by_field_name("declarator"), node.child_by_field_name("parameters")) {
                let decl_str = transpile_node(declarator, source, symbols);
                let params_str = transpile_node(parameters, source, symbols);
                format!("{} {}", decl_str, params_str)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "parameter_declaration" => {
            if let Some(type_node) = node.child_by_field_name("type") {
                let type_str = transpile_node(type_node, source, symbols);
                if let Some(decl) = node.child_by_field_name("declarator") {
                    let decl_str = transpile_node(decl, source, symbols);
                    format!("{} {}", type_str, decl_str)
                } else {
                    type_str
                }
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "compound_statement" => {
            let mut out = String::from("{\n");
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "{" || child.kind() == "}" { continue; }
                out.push_str("    ");
                out.push_str(&transpile_node(child, source, symbols));
                out.push('\n');
            }
            out.push('}');
            out
        }

        "expression_statement" => {
            if let Some(expr) = node.child(0) {
                let expr_str = transpile_node(expr, source, symbols);
                if expr_str.ends_with(';') {
                    expr_str
                } else {
                    format!("{};", expr_str)
                }
            } else {
                ";".to_string()
            }
        }

        "comma_expression" => {
            if let (Some(left), Some(right)) = (node.child_by_field_name("left"), node.child_by_field_name("right")) {
                let left_str = transpile_node(left, source, symbols);
                let right_str = transpile_node(right, source, symbols);
                
                fn is_in_statement_context(n: tree_sitter::Node) -> bool {
                    if let Some(parent) = n.parent() {
                        match parent.kind() {
                            "expression_statement" => true,
                            "comma_expression" => is_in_statement_context(parent),
                            _ => false,
                        }
                    } else {
                        false
                    }
                }

                if is_in_statement_context(node) {
                    format!("{};\n{}", left_str.trim_end_matches(';'), right_str)
                } else {
                    format!("/* TODO: Comma in expression context */ ({}, {})", left_str, right_str)
                }
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "while_statement" => {
            if let (Some(condition), Some(body)) = (node.child_by_field_name("condition"), node.child_by_field_name("body")) {
                let cond_str = transpile_node(condition, source, symbols);
                let mut body_str = transpile_node(body, source, symbols);
                
                if body.kind() != "compound_statement" {
                    body_str = format!("{{\n    {}\n}}", body_str.replace("\n", "\n    "));
                }
                format!("while {} {}", cond_str, body_str)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "if_statement" => {
            if let (Some(cond_node), Some(consequence_node)) = (node.child_by_field_name("condition"), node.child_by_field_name("consequence")) {
                let alternative_node = node.child_by_field_name("alternative");
                let cond_str = transpile_node(cond_node, source, symbols);
                let mut consequence_str = transpile_node(consequence_node, source, symbols);
                if consequence_node.kind() != "compound_statement" {
                    consequence_str = format!("{{\n    {}\n}}", consequence_str.replace("\n", "\n    "));
                }
                
                if let Some(alt_node) = alternative_node {
                    let mut alt_str = transpile_node(alt_node, source, symbols);
                    if alt_node.kind() != "compound_statement" && alt_node.kind() != "if_statement" {
                        alt_str = format!("{{\n    {}\n}}", alt_str.replace("\n", "\n    "));
                    }
                    format!("if {} {} else {}", cond_str, consequence_str, alt_str)
                } else {
                    format!("if {} {}", cond_str, consequence_str)
                }
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "for_statement" => {
            if let Some(body) = node.child_by_field_name("body") {
                let initializer = node.child_by_field_name("initializer");
                let condition = node.child_by_field_name("condition");
                let update = node.child_by_field_name("update");
                
                let init_str = initializer.map(|n| transpile_node(n, source, symbols)).unwrap_or_default();
                let cond_str = condition.map(|n| transpile_node(n, source, symbols)).unwrap_or_default();
                let upd_str = update.map(|n| transpile_node(n, source, symbols)).unwrap_or_default();
                
                let mut body_str = transpile_node(body, source, symbols);
                if body.kind() != "compound_statement" {
                    body_str = format!("{{\n    {}\n}}", body_str.replace("\n", "\n    "));
                }
                
                format!("for ({}; {}; {}) {}", init_str, cond_str, upd_str, body_str)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "binary_expression" => {
            if let (Some(left), Some(right)) = (node.child_by_field_name("left"), node.child_by_field_name("right")) {
                let operator = node.child_by_field_name("operator").map(|n| n.utf8_text(source).unwrap_or("")).unwrap_or("");
                let mut left_str = transpile_node(left, source, symbols);
                let mut right_str = transpile_node(right, source, symbols);
                
                if operator == "+" {
                    if left.kind() == "string_literal" {
                        left_str = format!("(char*){}", left_str);
                    } else if right.kind() == "string_literal" {
                        right_str = format!("(char*){}", right_str);
                    }
                }
                
                let parent_is_parenthesized = node.parent().map(|p| p.kind() == "parenthesized_expression").unwrap_or(false);
                let parenthesize = if parent_is_parenthesized {
                    false
                } else {
                    match operator {
                        "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                            left.kind() == "binary_expression" || right.kind() == "binary_expression"
                        }
                        _ => true,
                    }
                };
                
                if parenthesize {
                    format!("({} {} {})", left_str, operator, right_str)
                } else {
                    format!("{} {} {}", left_str, operator, right_str)
                }
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "switch_statement" => {
            if let (Some(condition), Some(body)) = (node.child_by_field_name("condition"), node.child_by_field_name("body")) {
                let cond_str = transpile_node(condition, source, symbols);
                let body_str = transpile_node(body, source, symbols);
                format!("switch {} {}", cond_str, body_str)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "case_statement" => {
            if let Some(value_node) = node.child_by_field_name("value") {
                let value_str = transpile_node(value_node, source, symbols);
                
                let mut cursor = node.walk();
                let mut statements = Vec::new();
                let mut found_colon = false;
                for child in node.children(&mut cursor) {
                    if found_colon {
                        statements.push(child);
                    } else if child.kind() == ":" {
                        found_colon = true;
                    }
                }
                
                let has_real_statements = statements.iter().any(|&c| c.kind() != "comment");
                if !has_real_statements {
                    let mut out = format!("case {}:", value_str);
                    for &stmt in &statements {
                        let stmt_str = transpile_node(stmt, source, symbols);
                        if !stmt_str.trim().is_empty() {
                            out.push_str(" ");
                            out.push_str(&stmt_str);
                        }
                    }
                    out
                } else {
                    let (statements_str, _) = transpile_switch_case_statements(&statements, source, symbols);
                    format!("case {}:\n{}", value_str, statements_str)
                }
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "default_statement" => {
            let mut cursor = node.walk();
            let mut statements = Vec::new();
            let mut found_colon = false;
            for child in node.children(&mut cursor) {
                if found_colon {
                    statements.push(child);
                } else if child.kind() == ":" {
                    found_colon = true;
                }
            }
            
            let (statements_str, _) = transpile_switch_case_statements(&statements, source, symbols);
            if statements_str.is_empty() {
                "default:".to_string()
            } else {
                format!("default:\n{}", statements_str)
            }
        }

        "call_expression" => {
            if let (Some(function), Some(arguments)) = (node.child_by_field_name("function"), node.child_by_field_name("arguments")) {
                let func_str = transpile_node(function, source, symbols);
                let args_str = transpile_node(arguments, source, symbols);
                format!("{}{}", func_str, args_str)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "argument_list" | "parameter_list" => {
            let mut cursor = node.walk();
            let mut elements = Vec::new();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    elements.push(transpile_node(child, source, symbols));
                }
            }
            format!("({})", elements.join(", "))
        }

        "initializer_list" => {
            let mut cursor = node.walk();
            let mut elements = Vec::new();
            for child in node.children(&mut cursor) {
                if child.kind() != "{" && child.kind() != "}" && child.kind() != "," {
                    elements.push(transpile_node(child, source, symbols));
                }
            }
            if elements.is_empty() {
                "{}".to_string()
            } else {
                format!("{{ {} }}", elements.join(", "))
            }
        }

        "parenthesized_expression" => {
            let mut cursor = node.walk();
            let mut expr_str = String::new();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" {
                    expr_str = transpile_node(child, source, symbols);
                }
            }
            format!("({})", expr_str)
        }

        "field_expression" => {
            if let (Some(argument), Some(field)) = (node.child_by_field_name("argument"), node.child_by_field_name("field")) {
                let lhs = transpile_node(argument, source, symbols);
                let rhs = transpile_node(field, source, symbols);
                format!("{}.{}", lhs, rhs)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "array_declarator" => {
            if let Some(decl) = node.child_by_field_name("declarator") {
                transpile_node(decl, source, symbols)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "pointer_declarator" => {
            if let Some(decl) = node.child_by_field_name("declarator") {
                format!("*{}", transpile_node(decl, source, symbols))
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "pointer_expression" => {
            if node.child_count() == 2 {
                let op = transpile_node(node.child(0).unwrap(), source, symbols);
                let operand = transpile_node(node.child(1).unwrap(), source, symbols);
                format!("{}{}", op, operand)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "type_qualifier" => {
            String::new()
        }

        "primitive_type" | "type_identifier" => {
            let text = node.utf8_text(source).unwrap_or("").trim();
            if let Some(converted) = convert_c_type(text) {
                converted
            } else if let Some(renamed) = symbols.types.get(text) {
                renamed.clone()
            } else {
                text.to_string()
            }
        }

        "preproc_include" => {
            if let Some(path_node) = node.child_by_field_name("path") {
                let path_str = path_node.utf8_text(source).unwrap_or("");
                if path_str.starts_with('<') {
                    match path_str {
                        "<stdio.h>" => {
                            concat!(
                                "extern fn int scanf(char* format, ...);\n",
                                "extern fn int printf(char* format, ...);\n",
                                "extern fn int snprintf(char* s, usz n, char* format, ...);\n",
                                "extern fn int sprintf(char* s, char* format, ...);\n",
                                "extern fn int sscanf(char* s, char* format, ...);\n",
                                "extern fn int fprintf(void* stream, char* format, ...);\n",
                                "extern fn int fscanf(void* stream, char* format, ...);\n",
                                "extern fn void* fopen(char* filename, char* mode);\n",
                                "extern fn int fclose(void* stream);\n",
                                "extern fn int feof(void* stream);\n",
                                "extern fn int ferror(void* stream);\n",
                                "extern fn int fgetc(void* stream);\n",
                                "extern fn char* fgets(char* s, int n, void* stream);\n",
                                "extern fn int fputc(int c, void* stream);\n",
                                "extern fn int fputs(char* s, void* stream);\n",
                                "extern fn usz fread(void* ptr, usz size, usz count, void* stream);\n",
                                "extern fn usz fwrite(void* ptr, usz size, usz count, void* stream);\n",
                                "extern fn int fseek(void* stream, long offset, int whence);\n",
                                "extern fn long ftell(void* stream);\n",
                                "extern fn void rewind(void* stream);\n",
                                "extern fn int getc(void* stream);\n",
                                "extern fn int getchar();\n",
                                "extern fn int putc(int c, void* stream);\n",
                                "extern fn int putchar(int c);\n",
                                "extern fn int puts(char* s);\n",
                                "extern fn int remove(char* filename);\n",
                                "extern fn int rename(char* old_name, char* new_name);"
                            ).to_string()
                        }
                        "<stdlib.h>" | "<string.h>" => {
                            "import libc;".to_string()
                        }
                        _ => format!("// TODO: Find C3 equivalent for {}", path_str),
                    }
                } else if path_str.starts_with('"') {
                    let file_name = path_str.trim_matches('"');
                    let module_name = file_name.strip_suffix(".h").unwrap_or(file_name);
                    format!("import {};", module_name)
                } else {
                    String::new()
                }
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "preproc_def" => {
            if let (Some(name_node), Some(value_node)) = (node.child_by_field_name("name"), node.child_by_field_name("value")) {
                let name_str = name_node.utf8_text(source).unwrap_or("").trim();
                let value_str = value_node.utf8_text(source).unwrap_or("").trim();
                let c3_name = symbols.constants.get(name_str).cloned().unwrap_or_else(|| name_str.to_string());
                format!("alias {} = {};", c3_name, value_str)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "init_declarator" => {
            if let (Some(decl), Some(val)) = (node.child_by_field_name("declarator"), node.child_by_field_name("value")) {
                let decl_str = transpile_node(decl, source, symbols);
                let mut val_str = transpile_node(val, source, symbols);
                
                let is_pointer = is_pointer_declarator(decl);
                if is_pointer {
                    if let Some(array_name) = get_raw_identifier_name(val, source) {
                        if symbols.arrays.contains(&array_name) {
                            val_str = format!("&{}", val_str);
                        }
                    }
                }
                format!("{} = {}", decl_str, val_str)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "declaration" | "field_declaration" => {
            if let Some(c3_const_enum) = transpile_const_enum(node, source, symbols) {
                return c3_const_enum;
            }
            
            let mut cursor = node.walk();
            let declarator_nodes: Vec<tree_sitter::Node> = node
                .children_by_field_name("declarator", &mut cursor)
                .collect();

            if declarator_nodes.is_empty() {
                let mut out = String::new();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == ";" {
                        continue;
                    }
                    out.push_str(&transpile_node(child, source, symbols));
                    out.push(' ');
                }
                out.replace(" ,", ",").trim().to_string()
            } else {
                let first_decl_start = declarator_nodes[0].start_byte();
                let mut type_part_out = String::new();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.start_byte() < first_decl_start {
                        type_part_out.push_str(&transpile_node(child, source, symbols));
                        type_part_out.push(' ');
                    }
                }
                let type_part = type_part_out.replace(" ,", ",").trim().to_string();

                let mut declarations = Vec::new();
                for decl_node in declarator_nodes {
                    let has_value = decl_node.kind() == "init_declarator";
                    
                    let mut custom_type_part = type_part.clone();
                    if let Some(arr_node) = find_array_declarator(decl_node) {
                        let (dims, _) = get_array_dimensions(arr_node, source, symbols);
                        if !dims.is_empty() {
                            let dims_joined = dims
                                .iter()
                                .map(|s| if s.is_empty() { "*" } else { s })
                                .collect::<Vec<_>>()
                                .join("][");
                            custom_type_part = format!("{}[{}]", type_part, dims_joined);
                        }
                    }

                    let decl_str = transpile_node(decl_node, source, symbols);
                    if has_value {
                        declarations.push(format!("{} {};", custom_type_part, decl_str));
                    } else {
                        declarations.push(format!("{} {} @noinit;", custom_type_part, decl_str));
                    }
                }

                let separator = "\n    ";
                declarations.join(separator)
            }
        }

        "type_definition" => {
            let type_node = node.child_by_field_name("type");
            let decl_node = node.child_by_field_name("declarator");

            if let (Some(t), Some(d)) = (type_node, decl_node) {
                let alias_name = if let Some(id) = extract_declarator_identifier(d) {
                    let raw = id.utf8_text(source).unwrap_or("");
                    symbols.types.get(raw).cloned().unwrap_or_else(|| raw.to_string())
                } else {
                    transpile_node(d, source, symbols)
                };

                fn count_pointer_depth(n: tree_sitter::Node) -> usize {
                    if n.kind() == "pointer_declarator" {
                        1 + n.child_by_field_name("declarator")
                            .map(|inner| count_pointer_depth(inner))
                            .unwrap_or(0)
                    } else {
                        0
                    }
                }

                let stars = "*".repeat(count_pointer_depth(d));
                let base_type = transpile_node(t, source, symbols);
                format!("alias {} = {}{};", alias_name, base_type, stars)
            } else {
                node.utf8_text(source).unwrap_or("").to_string()
            }
        }

        "comment" | "string_literal" | "char_literal" | "system_lib_string" | "number_literal" => {
            node.utf8_text(source).unwrap_or("").to_string()
        }

        _ => {
            if node.child_count() == 0 {
                let text = node.utf8_text(source).unwrap_or("");
                
                if node.kind() == "identifier" || node.kind() == "field_identifier" {
                    if let Some(renamed) = symbols.functions.get(text) {
                        return renamed.clone();
                    }
                    if let Some(renamed) = symbols.variables.get(text) {
                        return renamed.clone();
                    }
                    if let Some(renamed) = symbols.constants.get(text) {
                        return renamed.clone();
                    }
                }
                text.to_string()
            } else {
                let mut out = String::new();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    out.push_str(&transpile_node(child, source, symbols));
                    out.push(' '); 
                }
                
                out.replace(" ;", ";")
                    .replace(" ,", ",")
                    .replace("( ", "(")
                    .replace(" )", ")")
                    .trim()
                    .to_string()
            }
        }
    }
}

fn populate_symbols(node: tree_sitter::Node, source: &[u8], symbols: &mut SymbolTable) {
    let kind = node.kind();
    
    match kind {
        "struct_specifier" | "enum_specifier" | "union_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(c_name) = name_node.utf8_text(source) {
                    symbols.register_type(c_name);
                }
            }
        }
        "enumerator" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(c_name) = name_node.utf8_text(source) {
                    symbols.register_constant(c_name);
                }
            }
        }
        "type_definition" => {
            if let Some(decl) = node.child_by_field_name("declarator") {
                if let Some(id) = extract_declarator_identifier(decl) {
                    if let Ok(c_name) = id.utf8_text(source) {
                        symbols.register_type(c_name);
                    }
                }
            }
        }
        "function_definition" => {
            if let Some(decl) = node.child_by_field_name("declarator") {
                if let Some(id) = extract_declarator_identifier(decl) {
                    if let Ok(c_name) = id.utf8_text(source) {
                        symbols.register_function(c_name);
                    }
                }
            }
        }
        "parameter_declaration" => {
            if let Some(decl) = node.child_by_field_name("declarator") {
                if let Some(id) = extract_declarator_identifier(decl) {
                    if let Ok(c_name) = id.utf8_text(source) {
                        symbols.register_variable(c_name);
                    }
                }
            }
        }
        "declaration" => {
            let text = node.utf8_text(source).unwrap_or("");
            if text.starts_with("const") && text.contains('{') && !text.contains('=') {
                if let Some(before_brace) = text.split('{').next() {
                    let parts: Vec<&str> = before_brace.split_whitespace().collect();
                    if parts.len() >= 2 && parts[0] == "const" {
                        let c_name = parts[1];
                        symbols.register_type(c_name);
                        
                        if let Some(braced_part) = text.split_once('{') {
                            if let Some(body_content) = braced_part.1.split_once('}') {
                                for item in body_content.0.split(',') {
                                    let trimmed = item.trim();
                                    if !trimmed.is_empty() {
                                        let name = trimmed.split('=').next().unwrap().trim();
                                        symbols.register_constant(name);
                                    }
                                }
                            }
                        }
                    }
                }
                return; 
            }

            let mut cursor = node.walk();
            let declarators: Vec<tree_sitter::Node> = node
                .children_by_field_name("declarator", &mut cursor)
                .collect();
                
            let is_const = {
                let mut has_const = false;
                let mut cursor2 = node.walk();
                for child in node.children(&mut cursor2) {
                    if child.kind() == "type_qualifier" && child.utf8_text(source).unwrap_or("") == "const" {
                        has_const = true;
                        break;
                    }
                }
                has_const
            };

            for decl in declarators {
                if let Some(id) = extract_declarator_identifier(decl) {
                    if let Ok(c_name) = id.utf8_text(source) {
                        if is_const {
                            symbols.register_constant(c_name);
                        } else {
                            symbols.register_variable(c_name);
                            if find_array_declarator(decl).is_some() {
                                symbols.arrays.insert(c_name.to_string());
                            }
                        }
                    }
                }
            }
        }
        "preproc_def" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(c_name) = name_node.utf8_text(source) {
                    symbols.register_constant(c_name);
                }
            }
        }
        _ => {}
    }
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        populate_symbols(child, source, symbols);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path to C source file>", args[0]);
        std::process::exit(1);
    }
    let c_source = std::fs::read_to_string(&args[1]).expect("Failed to read C source file");
    let c_source = c_source.as_str();

    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_c::LANGUAGE; 
    parser.set_language(&language.into()).expect("Error loading C grammar");

    let tree = parser.parse(c_source, None).expect("Failed to parse C code");

    let root_node = tree.root_node();
    let source_bytes = c_source.as_bytes();

    let mut symbols = SymbolTable {
        types: HashMap::new(),
        functions: HashMap::new(),
        variables: HashMap::new(),
        constants: HashMap::new(),
        arrays: HashSet::new(),
    };
    
    populate_symbols(root_node, source_bytes, &mut symbols);

    println!("{}", root_node.to_sexp());

    let c3_output = transpile_node(root_node, source_bytes, &symbols);
    
    let output_path = std::path::Path::new(&args[1]).with_extension("c3");
    std::fs::write(&output_path, &c3_output).expect("Failed to write C3 output file");
    println!("C3 output written to: {}", output_path.display());
}