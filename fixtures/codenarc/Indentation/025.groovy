
class MyClass {
    void doStuff(String name) {
        String otherName = name
        doWith(
            name,
            { String someParam ->
                    println someParam  // note this extra level of indent; allowed
            }
        )

         println 123                   // violation
        doWith(
              name,                    // wrong column, but not a statement -- ignored
            { String someParam ->
             println someParam         // violation
                  println 123          // violation
            println 999                // allowed
                println 999            // allowed
                    println 999        // allowed
                        println 123    // too far; violation
            }
        )
    }
}
        