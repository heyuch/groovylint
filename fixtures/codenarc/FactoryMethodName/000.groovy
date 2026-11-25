
            class MyClass {

                def doSomething() {}
                def make() {}
                def makeSomething() {}
                @Override build() { } // OK, overriding
            }

            class WidgetBuilder {
                def build() {}
                def buildSomething() {}
            }
        