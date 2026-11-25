
            class MyClass {
                def mySession = servletContext

                def edit = {
                    println "amount=${servletContext.amount}"
                }
            }
        