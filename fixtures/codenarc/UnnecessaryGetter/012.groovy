
            Mock {
                getSomeData()
            }
            Stub(1,2,3) {
                getData2()
            }
            "${'Stub'}"(MyClass) {
                getData3()
            }
            def closure = { getData4() }
            Mock({ getData5() }, 1234)      // 2nd param is not a Closure
            Spy {
                getMoreData()
            }
        