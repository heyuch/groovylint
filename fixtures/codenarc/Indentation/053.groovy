
def foo() {
    1
        .tap {
            println("test")
            if (Boolean.valueOf("true")) {
                println("test")
            }
        }
        .tap { }
}
        