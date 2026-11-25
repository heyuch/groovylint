
            class MyClass {
                def name = "Joe" + new Date().toString()

                void run() {
                    Object object = 1
                    def string = 'some string' + object.toString()
                    def withinGString = "processing: ${'prefix' + object.toString()}"

                    def bigString = 'some string' + new Date() + object.toString()      // not a violation; known limitation
                    def other = 123 + object.toString().toInteger()     // not a violation
                }
            }
        