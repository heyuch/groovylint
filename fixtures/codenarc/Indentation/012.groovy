
class MyAstVisitor extends OtherAstVisitor {
    MyAstVisitor(String name) {
        super(name)
    }
    MyAstVisitor(int count) { println 'int' }
    MyAstVisitor() {
        super()
    }
    MyAstVisitor() {
        this(0)
    }
}
        