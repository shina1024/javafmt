use super::{format_lexed, trim_trailing_whitespace};
use crate::lexer;

fn format_source(source: &str) -> String {
    let lexed = lexer::lex(source);
    format_lexed(&lexed)
}

#[test]
fn formats_simple_class_body() {
    let source = "class A{int add(int a,int b){return a+b;}}";
    let formatted = format_source(source);
    assert_eq!(
        formatted,
        "class A {\n  int add(int a, int b) {\n    return a + b;\n  }\n}\n"
    );
}

#[test]
fn formats_string_and_char_literals() {
    let source = "class A{String s=\"x\";char c='y';}";
    let formatted = format_source(source);
    assert_eq!(
        formatted,
        "class A {\n  String s = \"x\";\n  char c = 'y';\n}\n"
    );
}

#[test]
fn formats_ternary_expression() {
    let source = "class A{int x(){return true?1:2;}}";
    let formatted = format_source(source);
    assert_eq!(
        formatted,
        "class A {\n  int x() {\n    return true ? 1 : 2;\n  }\n}\n"
    );
}

#[test]
fn formats_annotation_and_comment() {
    let source = "class A{@Override public String toString(){//x\nreturn \"x\";}}";
    let formatted = format_source(source);
    assert_eq!(
        formatted,
        "class A {\n  @Override\n  public String toString() { // x\n    return \"x\";\n  }\n}\n"
    );
}

#[test]
fn keeps_generic_without_spaces_around_angle() {
    let source = "class A{java.util.List<String> xs;}";
    let formatted = format_source(source);
    assert_eq!(formatted, "class A {\n  java.util.List<String> xs;\n}\n");
}

#[test]
fn keeps_operator_spacing_inside_parentheses() {
    let source =
        "class A{int f(int n){for(int i=0;i<n;i++){if(i%2==0){continue;}else{break;}}return n;}}";
    let formatted = format_source(source);
    assert!(formatted.contains("i < n"));
    assert!(formatted.contains("i % 2 == 0"));
}

#[test]
fn joins_catch_and_finally() {
    let source = "class A{void f(){try{foo();}catch(Exception e){bar();}finally{baz();}}void foo(){}void bar(){}void baz(){}}";
    let formatted = format_source(source);
    assert!(formatted.contains("} catch (Exception e) {"));
    assert!(formatted.contains("} finally {"));
}

#[test]
fn formats_switch_case_labels() {
    let source = "class A{void f(){switch(x){case 1:foo();break;default:bar();}}int x;void foo(){}void bar(){}}";
    let formatted = format_source(source);
    assert!(formatted.contains("case 1:\n"));
    assert!(formatted.contains("default:\n"));
}

#[test]
fn keeps_non_sealed_keyword() {
    let source = "class A{non-sealed class B{}}";
    let formatted = format_source(source);
    assert!(formatted.contains("non-sealed class B"));
}

#[test]
fn adds_blank_line_between_block_members_in_type_body() {
    let source = "class A{void f(){}void g(){}}";
    let formatted = format_source(source);
    assert!(formatted.contains("void f() {}\n\n  void g() {}"));
}

#[test]
fn breaks_block_lambda_after_assignment() {
    let source = "class A{void f(){Runnable r=()->{x();};}}";
    let formatted = format_source(source);
    assert!(formatted.contains("Runnable r =\n"));
    assert!(formatted.contains("() -> {\n"));
}

#[test]
fn does_not_merge_binary_plus_and_unary_plus_into_increment() {
    let source = "class A{int f(int x){return -x + +x + ~x;}}";
    let formatted = format_source(source);
    assert!(formatted.contains("return -x + +x + ~x;"));
    assert!(!formatted.contains("x++"));
}

#[test]
fn formats_switch_arrow_labels() {
    let source = "class A{void f(int x){switch(x){case 1->System.out.println(1);default->{System.out.println(0);}}}}";
    let formatted = format_source(source);
    assert!(formatted.contains("case 1 -> System.out.println(1);"));
    assert!(formatted.contains("default -> {\n"));
}

#[test]
fn formats_try_with_resources_parentheses_spacing() {
    let source = "class A{void f(){try(var in=new java.io.ByteArrayInputStream(new byte[0])){in.read();}catch(java.io.IOException e){throw new RuntimeException(e);}}}";
    let formatted = format_source(source);
    assert!(formatted.contains("try (var in ="));
}

#[test]
fn keeps_do_while_on_single_line_join() {
    let source = "class A{void f(){do{x();}while(cond());}void x(){}boolean cond(){return true;}}";
    let formatted = format_source(source);
    assert!(formatted.contains("} while (cond());"));
}

#[test]
fn keeps_array_initializer_inline_for_short_literal() {
    let source = "class A{void f(){int[] a=new int[]{1,2,3};}}";
    let formatted = format_source(source);
    assert!(formatted.contains("new int[] {1, 2, 3};"));
}

#[test]
fn keeps_plain_array_initializer_inline_for_short_literal() {
    let source = "class A{void f(){int[] a={1,2,3};}}";
    let formatted = format_source(source);
    assert!(formatted.contains("int[] a = {1, 2, 3};"));
}

#[test]
fn keeps_generic_method_invocation_without_extra_spaces() {
    let source = "class A{void f(){this.<String>m(\"x\");} <T> void m(T t){}}";
    let formatted = format_source(source);
    assert!(formatted.contains("this.<String>m(\"x\");"));
    assert!(formatted.contains("<T> void m(T t) {}"));
}

#[test]
fn spaces_shift_operators_as_binary_ops() {
    let source = "class A{void f(){int x=a>>b>>>c<<d;}}";
    let formatted = format_source(source);
    assert!(formatted.contains("int x = a >> b >>> c << d;"));
}

#[test]
fn keeps_switch_multi_labels_comma_spacing() {
    let source = "class A{int f(int x){return switch(x){case 1,2->1;default->0;};}}";
    let formatted = format_source(source);
    assert!(formatted.contains("case 1, 2 -> 1;"));
}

#[test]
fn keeps_switch_guard_method_call_spacing() {
    let source =
        "class A{void f(){var p=switch(x){case String s when s.length()>3->s;default->\"\";};}}";
    let formatted = format_source(source);
    assert!(formatted.contains("when s.length() > 3 -> s;"));
}

#[test]
fn keeps_scientific_notation_sign_tight() {
    let source = "class A{void f(){double d=1.23e-4;double e=2.0E+8;}}";
    let formatted = format_source(source);
    assert!(formatted.contains("double d = 1.23e-4;"));
    assert!(formatted.contains("double e = 2.0E+8;"));
}

#[test]
fn breaks_return_before_text_block_literal() {
    let source = "class A{String f(){return \"\"\"\nline1\nline2\n\"\"\";}}";
    let formatted = format_source(source);
    assert!(formatted.contains("return\n\"\"\"\nline1"));
}

#[test]
fn breaks_assignment_before_switch_expression_rhs() {
    let source =
        "class A{void f(){var p=switch(x){case String s when s.length()>3->s;default->\"\";};}}";
    let formatted = format_source(source);
    assert!(formatted.contains("var p =\n"));
    assert!(formatted.contains("switch (x) {"));
}

#[test]
fn breaks_long_chained_call_assignment() {
    let source = "class A{void f(){var x=java.util.stream.Stream.of(1,2,3,4,5).map(i->i+1).filter(i->i%2==0).sorted().toList();}}";
    let formatted = format_source(source);
    assert!(formatted.contains("var x =\n"));
    assert!(formatted.contains("\n            .map("));
}

#[test]
fn keeps_builder_seed_on_first_chain_line() {
    let source = "class A{void f(){var x=veryLongClient.fetch(someInput,SOME_REALLY_LONG_CONSTANT_NAME).toBuilder().setFoo(x).setBar(y).build();}}";
    let formatted = format_source(source);
    assert!(formatted.contains(
        "veryLongClient.fetch(someInput, SOME_REALLY_LONG_CONSTANT_NAME).toBuilder()\n            .setFoo(x)"
    ));
}

#[test]
fn breaks_long_return_chain() {
    let source = "class A{Object f(){return writtenVariables.stream().filter(var->deletedVariableIds.contains(var.getId())).collect(toImmutableList());}}";
    let formatted = format_source(source);
    assert!(formatted.contains("return writtenVariables.stream()\n"));
    assert!(formatted.contains("        .filter("));
    assert!(formatted.contains("        .collect(toImmutableList());"));
}

#[test]
fn breaks_long_qualified_call_arguments() {
    let source = "class A{void f(){logger.atDebug().log(\"Scratch Session Cleaner exiting. Number of deleted sessions: %d, names: %s\",deletedPersistentNames.size(),deletedPersistentNames);}}";
    let formatted = format_source(source);
    assert!(formatted.contains(".log(\n"));
    assert!(formatted.contains("%s\",\n"));
    assert!(formatted.contains("deletedPersistentNames.size(), deletedPersistentNames);"));
}

#[test]
fn breaks_long_unqualified_call_arguments() {
    let source = "class A{void f(){g(xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx);}}";
    let formatted = format_source(source);
    assert!(formatted.contains("g(\n"));
    assert!(formatted.contains(",\n"));
}

#[test]
fn forces_vertical_call_arguments_for_leading_or_multiple_calls() {
    let source = "class A{void f(){g(xxxxxxxxxxxxxxxxxxxxxxx(),xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx);h(xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxx(),xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxx(),xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx);}}";
    let formatted = format_source(source);
    assert!(
        formatted
            .contains("g(\n        xxxxxxxxxxxxxxxxxxxxxxx(),\n        xxxxxxxxxxxxxxxxxxxxxxxxx,")
    );
    assert!(
        formatted
            .contains("h(\n        xxxxxxxxxxxxxxxxxxxxxxxxx,\n        xxxxxxxxxxxxxxxxxxxxxxx(),")
    );
}

#[test]
fn forces_vertical_call_arguments_for_dense_broken_input_lines() {
    let source = "class A{void f(){g(\nxxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,\nxxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx,xxxxxxxxxxxxxxxxxxxxxxxxx);}}";
    let formatted = format_source(source);
    assert!(
        formatted
            .contains("g(\n        xxxxxxxxxxxxxxxxxxxxxxxxx,\n        xxxxxxxxxxxxxxxxxxxxxxxxx,")
    );
}

#[test]
fn normalizes_parameter_comments_and_breaks_long_comment_calls() {
    let source = "class A{void f(){f(/*a=*/ 1);g(/*a=*/ 1,/*b=*/ 1,/*c=*/ 1,/*d=*/ 1,/*e=*/ 1,/*f=*/ 1,/*g=*/ 1,/*h=*/ 1,/*i=*/ 1);h(/*xs...=*/ null);}}";
    let formatted = format_source(source);
    assert!(formatted.contains("f(/* a= */ 1);"));
    assert!(formatted.contains("g(\n"));
    assert!(formatted.contains("/* a= */ 1,\n"));
    assert!(formatted.contains("h(/* xs...= */ null);"));
}

#[test]
fn preserves_switch_comments_around_labels() {
    let source = "class A{int f(String v){return switch(v){case \"one\"->//x\n1;case \"two\"://y\n2;default->0;};}int g(String v){switch(v){case \"one\":return 1;//z\ncase \"two\":return 2;default:return 0;}}}";
    let formatted = format_source(source);
    assert!(formatted.contains("case \"one\" -> // x\n          1;"));
    assert!(formatted.contains("case \"two\": // y\n        2;"));
    assert!(formatted.contains("// z\n      case \"two\":"));
}

#[test]
fn preserves_blank_lines_between_fields_from_source() {
    let source =
        "class A{\n\nint a=1;\nint b=1;\n\nint c=1;\n/** Javadoc */\nint d=1;\n\nint e=1;\n\n}";
    let formatted = format_source(source);
    assert!(formatted.starts_with("class A {\n\n  int a = 1;"));
    assert!(formatted.contains("int b = 1;\n\n  int c = 1;"));
    assert!(formatted.contains("int d = 1;\n\n  int e = 1;"));
    assert!(formatted.ends_with("  int e = 1;\n}\n"));
}

#[test]
fn keeps_type_use_annotations_inline() {
    let source = "class A{@Deprecated public @Nullable Object f(){@Nullable Bar bar=bar();return null;}public @Deprecated Object g(){return null;}@Deprecated @Nullable A(){} @Nullable @Nullable Object h(){return null;} Object bar(){return null;} static class Bar{}}";
    let formatted = format_source(source);
    assert!(formatted.contains("@Deprecated\n  public @Nullable Object f() {"));
    assert!(formatted.contains("@Nullable Bar bar = bar();"));
    assert!(formatted.contains("public @Deprecated Object g() {"));
    assert!(formatted.contains("@Deprecated\n  @Nullable\n  A() {}"));
}

#[test]
fn formats_local_annotations_like_gjf() {
    let source = "class A{{@Foo final Object x;@Foo(1) final Object y;@Foo(x=1) final Object z;}}";
    let formatted = format_source(source);
    assert!(formatted.contains("@Foo final Object x;"));
    assert!(formatted.contains("@Foo(1)\n    final Object y;"));
    assert!(formatted.contains("@Foo(x = 1)\n    final Object z;"));
}

#[test]
fn breaks_argument_member_annotations_in_type_body() {
    let source = "class A{@SuppressWarnings({\"unchecked\",\"rawtypes\"}) void f(){}}";
    let formatted = format_source(source);
    assert!(formatted.contains("@SuppressWarnings({\"unchecked\", \"rawtypes\"})\n  void f() {"));
}

#[test]
fn breaks_override_annotation_before_member_signature() {
    let source = "class A{@Override String f(){return \"\";}}";
    let formatted = format_source(source);
    assert!(formatted.contains("@Override\n  String f() {"));
}

#[test]
fn breaks_top_level_annotations_before_module_declaration() {
    let source = "@A @B module m{}";
    let formatted = format_source(source);
    assert!(formatted.contains("@A\n@B\nmodule m {"));
}

#[test]
fn formats_try_with_multiple_resources_multiline() {
    let source = "class A{void f(){try(var in1=open();var in2=open2()){use(in1,in2);}catch(Exception e){x();}}java.io.InputStream open(){return null;}java.io.InputStream open2(){return null;}void use(java.io.InputStream a,java.io.InputStream b){}void x(){}}";
    let formatted = format_source(source);
    assert!(formatted.contains("try (var in1 = open();\n"));
    assert!(formatted.contains("var in2 = open2())"));
}

#[test]
fn keeps_annotation_array_inline_when_short() {
    let source = "class A{@SuppressWarnings({\"unchecked\",\"rawtypes\"}) void f(){}}";
    let formatted = format_source(source);
    assert!(formatted.contains("@SuppressWarnings({\"unchecked\", \"rawtypes\"})"));
}

#[test]
fn formats_enum_constants_on_separate_lines() {
    let source = "class A{enum E{A(1),B(2);final int n;E(int n){this.n=n;}}}";
    let formatted = format_source(source);
    assert!(formatted.contains("A(1),\n    B(2);"));
}

#[test]
fn formats_module_to_and_with_clauses_multiline() {
    let source = "module m.example{requires java.base;exports a.b;opens a.c to x.y,z.w;uses a.spi.S;provides a.spi.S with a.impl.SImpl;}";
    let formatted = format_source(source);
    assert!(formatted.contains("opens a.c to\n"));
    assert!(formatted.contains("x.y,\n"));
    assert!(formatted.contains("provides a.spi.S with\n"));
}

#[test]
fn formats_top_level_package_and_import_blank_lines() {
    let source = "package p;import java.util.List;class A{}";
    let formatted = format_source(source);
    assert!(formatted.starts_with("package p;\n\nimport java.util.List;\n\nclass A {}\n"));
}

#[test]
fn formats_statement_label_without_space_before_colon() {
    let source = "class A{void f(){outer:for(int i=0;i<1;i++){break outer;}}}";
    let formatted = format_source(source);
    assert!(formatted.contains("outer:\n"));
    assert!(!formatted.contains("outer :"));
}

#[test]
fn keeps_enum_constant_body_comma_on_same_line() {
    let source = "class A{enum E{A{int v(){return 1;}},B;}}";
    let formatted = format_source(source);
    assert!(formatted.contains("},\n    B;"));
}

#[test]
fn keeps_module_requires_group_compact() {
    let source =
        "module m.probe{requires transitive java.base;requires static java.sql;exports p.api;}";
    let formatted = format_source(source);
    assert!(formatted.contains(
        "requires transitive java.base;\n  requires static java.sql;\n\n  exports p.api;"
    ));
}

#[test]
fn keeps_annotation_interface_keyword_together() {
    let source = "class A{@interface B{}}";
    let formatted = format_source(source);
    assert!(formatted.contains("@interface B {"));
}

#[test]
fn keeps_modifier_and_annotation_interface_on_same_line() {
    let source = "class A{public @interface B{} private @interface C{} protected @interface D{}}";
    let formatted = format_source(source);
    assert!(formatted.contains("public @interface B {"));
    assert!(formatted.contains("private @interface C {}"));
    assert!(formatted.contains("protected @interface D {}"));
}

#[test]
fn keeps_block_comment_on_its_own_line_after_open_brace() {
    let source =
        "class A{void f(){do{/*x*/a();}while(cond());}void a(){}boolean cond(){return true;}}";
    let formatted = format_source(source);
    assert!(formatted.contains("do {\n        /*x*/\n      a();"));
}

#[test]
fn keeps_standalone_block_comment_on_its_own_line_before_type() {
    let source = "/** doc */class A{}";
    let formatted = format_source(source);
    assert_eq!(formatted, "/** doc */\nclass A {}\n");
}

#[test]
fn breaks_named_annotation_arguments_multiline() {
    let source = "class A{@Anno(values={1,2,3},name=\"x\") void f(){} @interface Anno{int[] values();String name();}}";
    let formatted = format_source(source);
    assert!(formatted.contains("@Anno(\n"));
    assert!(formatted.contains("values = {1, 2, 3},\n"));
    assert!(formatted.contains("name = \"x\")"));
}

#[test]
fn keeps_annotations_inline_inside_parameter_lists() {
    let source = "class A{record R<T>(@Deprecated int x,@Deprecated int... y){} void f(final @Deprecated String s){}}";
    let formatted = format_source(source);
    assert!(formatted.contains("record R<T>(@Deprecated int x, @Deprecated int... y) {}"));
    assert!(formatted.contains("void f(final @Deprecated String s) {}"));
}

#[test]
fn keeps_try_resource_annotations_inline() {
    let source = "class A{{try(@A final @B C c=c();){}}}";
    let formatted = format_source(source);
    assert!(formatted.contains("try (@A final @B C c = c(); ) {}"));
}

#[test]
fn breaks_long_generic_assignment_rhs() {
    let source = "class A{void f(){var m=java.util.Map.<String,java.util.List<java.util.Set<Integer>>>of(\"k\",java.util.List.of(java.util.Set.of(1,2)));}}";
    let formatted = format_source(source);
    assert!(formatted.contains("var m =\n"));
}

#[test]
fn keeps_generic_types_tight_inside_parameter_lists() {
    let source = "class A{record R<T>(){} int f(R<T> other,java.util.Map<String,java.util.List<T>> xs){return 0;}}";
    let formatted = format_source(source);
    assert!(formatted.contains("int f(R<T> other, java.util.Map<String, java.util.List<T>> xs) {"));
    assert!(!formatted.contains("R < T >"));
    assert!(!formatted.contains("List < T >"));
}

#[test]
fn keeps_diamond_operator_without_spaces() {
    let source = "class A{record R<T>(T x){} R<Integer> r=new R<>(1);} ";
    let formatted = format_source(source);
    assert!(formatted.contains("new R<>(1);"));
    assert!(!formatted.contains("R < >"));
}

#[test]
fn attaches_line_comment_after_open_brace() {
    let source = "class A{void f(){//@x\nx();}}";
    let formatted = format_source(source);
    assert!(formatted.contains("void f() { // @x"));
}

#[test]
fn attaches_block_comment_to_previous_statement_when_inline() {
    let source = "class A{void f(){x();/*y*/z();}}";
    let formatted = format_source(source);
    assert!(formatted.contains("x(); /*y*/\n"));
    assert!(formatted.contains("z();"));
}

#[test]
fn trims_spaces_before_newline() {
    let output = trim_trailing_whitespace("class A {}   \n");
    assert_eq!(output, "class A {}\n");
}

#[test]
fn keeps_internal_whitespace() {
    let output = trim_trailing_whitespace("a  b\n");
    assert_eq!(output, "a  b\n");
}
