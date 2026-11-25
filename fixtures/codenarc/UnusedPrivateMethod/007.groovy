
             class MyClass {
                 private static def methodWithParameters(a,b) { a >> b }

                 def x = new Class() {
                     def call() {
                         methodWithParameters(10,1)
                     }
                 }
             } 