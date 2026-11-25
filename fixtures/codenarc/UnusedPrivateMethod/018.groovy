
            class MyClass {
                private int countStuff() { return 99 }
                int somePublicMethod() { }
                def abc = 'abc'
                private String getName() { 'abc' }
                private getPrice() { 0.0 }

                def doStuff() {
                    def count = countStuff()
                    def newName = this.getName()
                }

                def myClosure = { println "price is ${getPrice()}" }
            }
        