
                class MyClass {
                    def myMethod() {
                        def greeting = fullname?:'you'
                        def f = funds.collect {it.fundSortOrder}?:[]
                        assert model.UserID == expectedModel.UserID?:null
                        def tripleElvis = fullname ?:lastname ?: middleName?:'me'
                    }
                }
            