
            class MyClass {
                def a = (double) Math.random()
                def b = (Double) Math.random()
                double c = Math.random()
                Double d = Math.random()
                def e = (Math.random()) as double
                def f = (Math.random()) as Double
                def g = Math.random()
            }

            Double intMethod() {
                if (foo) return Math.random()
            }
            double integerMethod() {
                if (foo) return Math.random()
            }
            def defMethod() {
                if (foo) return Math.random()
            }
        