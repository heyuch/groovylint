
            final class MyClass1 {
                public method1() {}
                def method2() {}
                @PackageScope def method3() {}
                private def method3() {}
            }
            class MyClass2 {
                protected def method() {}
                protected def closure = {}
                protected String property
            }

            final class MyClass {
                @Override
                protected def methodName() {}
            }
        