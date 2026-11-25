
            class MyClass {
                def myClosure = {
                    if (error) {
                        throw new java.lang.Error('something bad')
                    }
                }
            }
        