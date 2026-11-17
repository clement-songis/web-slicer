// Génère les STEP de test du maillage STEP (ffi_step_mesh) avec OCCT :
//  - cylinder.step : cylindre Ø20 × 20 (surfaces courbes : la déflexion compte) ;
//  - compound.step : COMPOUND d'un cube 20 et d'un cylindre décalé (découpe des compounds).
// Compilation : voir make_ref.sh.
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRep_Builder.hxx>
#include <STEPControl_Writer.hxx>
#include <TopoDS_Compound.hxx>
#include <gp_Ax2.hxx>
#include <STEPCAFControl_Writer.hxx>
#include <TDocStd_Document.hxx>
#include <XCAFApp_Application.hxx>
#include <XCAFDoc_DocumentTool.hxx>
#include <XCAFDoc_ShapeTool.hxx>

// Compound écrit comme forme SIMPLE (pas assemblage) : à la lecture, le label
// porte un TopAbs_COMPOUND sans composants — cas « split compound » d'Orca.
static int write_simple_compound(const TopoDS_Shape &shape, const char *path)
{
    Handle(XCAFApp_Application) app = XCAFApp_Application::GetApplication();
    Handle(TDocStd_Document) doc;
    app->NewDocument("MDTV-XCAF", doc);
    Handle(XCAFDoc_ShapeTool) tool = XCAFDoc_DocumentTool::ShapeTool(doc->Main());
    tool->AddShape(shape, /*makeAssembly=*/Standard_False);
    STEPCAFControl_Writer writer;
    if (!writer.Transfer(doc, STEPControl_AsIs))
        return 1;
    return writer.Write(path) == IFSelect_RetDone ? 0 : 1;
}

static int write(const TopoDS_Shape &shape, const char *path)
{
    STEPControl_Writer writer;
    if (writer.Transfer(shape, STEPControl_AsIs) != IFSelect_RetDone)
        return 1;
    return writer.Write(path) == IFSelect_RetDone ? 0 : 1;
}

int main(int argc, char **argv)
{
    const std::string dir = argc > 1 ? argv[1] : ".";
    TopoDS_Shape cyl = BRepPrimAPI_MakeCylinder(10.0, 20.0).Shape();
    if (write(cyl, (dir + "/cylinder.step").c_str()))
        return 1;
    BRep_Builder builder;
    TopoDS_Compound compound;
    builder.MakeCompound(compound);
    builder.Add(compound, BRepPrimAPI_MakeBox(20.0, 20.0, 20.0).Shape());
    builder.Add(compound, BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(40, 10, 0), gp_Dir(0, 0, 1)), 8.0, 15.0).Shape());
    return write_simple_compound(compound, (dir + "/compound.step").c_str());
}
