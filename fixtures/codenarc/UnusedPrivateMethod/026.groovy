
            class MyClass {
                private int countStuff() { }
                def varName = "countStuff"
                def other = this."${varName}"()     // can't see this
            }
        