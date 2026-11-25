
class MyAstVisitor extends OtherAstVisitor {
  MyAstVisitor() {
                this(23)                      // ignored; known issue
             println 123                      // violation
  }
    MyAstVisitor(int count) {
           super(Long, [String, Long], 'L')   // ignored; known issue
    }
}
        