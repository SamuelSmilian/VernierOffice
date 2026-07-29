use std::io::Cursor;
use vernier_office::backend::docx::DocxBackend;
use vernier_office::backend::pptx::PptxBackend;
use vernier_office::backend::Backend;
use vernier_office::lexer::lex;
use vernier_office::parser::parse;

/// Full pipeline: source -> lex -> parse -> docx backend
#[test]
fn test_full_pipeline_docx() {
    let source = concat!(
        "\\title{Test Document}\n",
        "\\author{Jane}\n\n",
        "\\heading{1}{Introduction}\n",
        "Hello world.\n\n",
        "\\begin{itemize}\n",
        "\\item First item\n",
        "\\item Second item\n",
        "\\end{itemize}\n"
    );

    let tokens = lex(source);
    let document = parse(tokens).expect("parse should succeed");
    let backend = DocxBackend;
    let mut buf = Cursor::new(Vec::new());
    backend.compile(&document, &mut buf).expect("compile should succeed");
    let data = buf.into_inner();

    // Verify it's a valid ZIP
    let cursor = Cursor::new(data);
    let mut zip = zip::ZipArchive::new(cursor).expect("should be valid zip");

    assert!(zip.by_name("word/document.xml").is_ok());
    assert!(zip.by_name("[Content_Types].xml").is_ok());
    assert!(zip.by_name("docProps/core.xml").is_ok());

    let doc_xml = {
        let mut f = zip.by_name("word/document.xml").unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s).unwrap();
        s
    };
    assert!(doc_xml.contains("Introduction"));
    assert!(doc_xml.contains("Hello world"));
    assert!(doc_xml.contains("First item"));
    assert!(doc_xml.contains("Second item"));
}

/// Full pipeline: source -> lex -> parse -> pptx backend
#[test]
fn test_full_pipeline_pptx() {
    let source = concat!(
        "\\title{Test Pres}\n",
        "\\author{Jane}\n\n",
        "\\begin{slide}\n",
        "\\slidetitle{Slide One}\n",
        "Content here.\n",
        "\\end{slide}\n"
    );

    let tokens = lex(source);
    let document = parse(tokens).expect("parse should succeed");
    let backend = PptxBackend;
    let mut buf = Cursor::new(Vec::new());
    backend.compile(&document, &mut buf).expect("compile should succeed");
    let data = buf.into_inner();

    let cursor = Cursor::new(data);
    let mut zip = zip::ZipArchive::new(cursor).expect("should be valid zip");

    assert!(zip.by_name("ppt/presentation.xml").is_ok());
    assert!(zip.by_name("ppt/slides/slide1.xml").is_ok());
    assert!(zip.by_name("ppt/slideMasters/slideMaster1.xml").is_ok());

    let slide_xml = {
        let mut f = zip.by_name("ppt/slides/slide1.xml").unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s).unwrap();
        s
    };
    assert!(slide_xml.contains("Slide One"));
    assert!(slide_xml.contains("Content here"));
}

/// Parse errors should be surfaced, not crash
#[test]
fn test_parse_error_surfaces() {
    let source = "\\heading{invalid}{Title}"; // non-numeric level
    let tokens = lex(source);
    let result = parse(tokens);
    assert!(result.is_err());
}

/// Lexer handles escape sequences in a real document context
#[test]
fn test_escape_sequences_in_context() {
    let source = "Use \\{braces\\} and \\\\ for backslash.";
    let tokens = lex(source);
    let document = parse(tokens).expect("parse should succeed");

    // Should produce a paragraph with the escaped characters as text
    assert_eq!(document.body.len(), 1);
}

/// DOCX backend handles document with slide elements gracefully
#[test]
fn test_docx_with_slides_skips_them() {
    let source = concat!(
        "\\title{Mixed}\n\n",
        "Paragraph before.\n\n",
        "\\begin{slide}\n",
        "\\slidetitle{A Slide}\n",
        "Slide content.\n",
        "\\end{slide}\n\n",
        "Paragraph after.\n"
    );

    let tokens = lex(source);
    let document = parse(tokens).expect("parse should succeed");
    let backend = DocxBackend;
    let mut buf = Cursor::new(Vec::new());
    backend.compile(&document, &mut buf).expect("compile should succeed");
    let data = buf.into_inner();

    let cursor = Cursor::new(data);
    let mut zip = zip::ZipArchive::new(cursor).unwrap();
    let doc_xml = {
        let mut f = zip.by_name("word/document.xml").unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s).unwrap();
        s
    };

    assert!(doc_xml.contains("Paragraph before"));
    assert!(doc_xml.contains("Paragraph after"));
    // Slide content should be absent
    assert!(!doc_xml.contains("Slide content"));
}

/// PPTX backend with metadata only produces a title slide
#[test]
fn test_pptx_empty_body_produces_title_slide() {
    let source = "\\title{Only Title}\n\\author{Writer}\n";
    let tokens = lex(source);
    let document = parse(tokens).expect("parse should succeed");
    let backend = PptxBackend;
    let mut buf = Cursor::new(Vec::new());
    backend.compile(&document, &mut buf).expect("compile should succeed");
    let data = buf.into_inner();

    let cursor = Cursor::new(data);
    let mut zip = zip::ZipArchive::new(cursor).unwrap();

    assert!(zip.by_name("ppt/slides/slide1.xml").is_ok());
    let slide_xml = {
        let mut f = zip.by_name("ppt/slides/slide1.xml").unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s).unwrap();
        s
    };
    assert!(slide_xml.contains("Only Title"));
}

/// Generate and validate both output formats from same document
#[test]
fn test_same_document_different_backends() {
    let source = concat!(
        "\\title{Dual Output}\n\n",
        "\\heading{1}{Section}\n",
        "Some text with \\em{emphasis}.\n"
    );

    let tokens = lex(source);
    let document = parse(tokens).expect("parse should succeed");

    // DOCX
    let mut docx_buf = Cursor::new(Vec::new());
    DocxBackend
        .compile(&document, &mut docx_buf)
        .expect("docx compile");

    // PPTX
    let mut pptx_buf = Cursor::new(Vec::new());
    PptxBackend
        .compile(&document, &mut pptx_buf)
        .expect("pptx compile");

    let docx_data = docx_buf.into_inner();
    let pptx_data = pptx_buf.into_inner();

    let mut docx_zip = zip::ZipArchive::new(Cursor::new(docx_data)).unwrap();
    let mut pptx_zip = zip::ZipArchive::new(Cursor::new(pptx_data)).unwrap();

    assert!(docx_zip.by_name("word/document.xml").is_ok());
    assert!(pptx_zip.by_name("ppt/presentation.xml").is_ok());
}
