
class MyClass {
    static {
        println "Static initializer"
          initializeMetaClass()            // wrong column
    }

    static { println "init" }

    static {
        [1, 2, 3].each { n ->
            println n
        }
    }

    // Instance initializer
    {
        println "Instance initializer"
      initialize(1)                        // wrong column
        initialize(2)
    }
}
        