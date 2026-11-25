
            class NoEntity {
                NoEntity save() {

                }

                static List<NoEntity> list(){
                    [new NoEntity()]
                }

            }

            class SomeService {

                void someMethod() {
                    List<NoEntity> list = NoEntity.list()
                    list.first().save()
                }

            }
        