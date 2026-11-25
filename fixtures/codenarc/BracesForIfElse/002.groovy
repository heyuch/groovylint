
            def myMethod1() {
                if (a &&
                        b) {
                }
             }

            def myMethod2() {
                if (rule.checkList &&
                        !isIgnoredOneElementList(expression) &&
                        !otherCheck() &&
                        someOtherCheck(expression)
                ) {
                    println 123
                }
             }

            def myMethod3() {
                if (flag1 &&
                        flag2
                        )      {
                }
             }
        