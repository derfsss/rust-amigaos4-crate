#include <datatypes/datatypes.h>
#include <datatypes/datatypesclass.h>
#include <datatypes/pictureclass.h>
#include <datatypes/textclass.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* datatypes.rs */
A(offsetof(struct DataType, dtn_Header) == 28);
A(offsetof(struct DataTypeHeader, dth_BaseName) == 4 && offsetof(struct DataTypeHeader, dth_Pattern) == 8);
A(offsetof(struct DataTypeHeader, dth_GroupID) == 16 && offsetof(struct DataTypeHeader, dth_ID) == 20 && offsetof(struct DataTypeHeader, dth_Priority) == 30);
A(sizeof(struct BitMapHeader) == 20 && offsetof(struct BitMapHeader, bmh_Depth) == 8 && offsetof(struct BitMapHeader, bmh_Compression) == 10);
A(offsetof(struct BitMapHeader, bmh_Transparent) == 12 && offsetof(struct BitMapHeader, bmh_XAspect) == 14 && offsetof(struct BitMapHeader, bmh_PageWidth) == 16);
A(sizeof(struct pdtBlitPixelArray) == 32 && offsetof(struct pdtBlitPixelArray, pbpa_PixelData) == 4 && offsetof(struct pdtBlitPixelArray, pbpa_Height) == 28);
A(PBPAFMT_RGB == 0 && PBPAFMT_RGBA == 1 && PBPAFMT_ARGB == 2 && PDTM_READPIXELARRAY == 0x661 && PDTM_WRITEPIXELARRAY == 0x660);
A(DTM_PROCLAYOUT == 0x602 && DTM_WRITE == 0x650 && DTST_RAM == 1 && DTST_FILE == 2 && DTST_MEMORY == 5 && PMODE_V43 == 1);
A(DTA_Dummy == TAG_USER + 0x1000 && DTA_TotalVert == DTA_Dummy + 13 && DTA_TotalHoriz == DTA_Dummy + 17 && DTA_GroupID == DTA_Dummy + 31);
A(DTA_SourceAddress == DTA_Dummy + 39 && DTA_SourceSize == DTA_Dummy + 40 && DTA_SourceType == DTA_Dummy + 101 && DTA_DataType == DTA_Dummy + 103);
A(DTA_NominalVert == DTA_Dummy + 124 && DTA_NominalHoriz == DTA_Dummy + 125);
A(PDTA_ModeID == DTA_Dummy + 200 && PDTA_BitMapHeader == DTA_Dummy + 201 && PDTA_NumColors == DTA_Dummy + 209 && PDTA_Remap == DTA_Dummy + 211 && PDTA_DestMode == DTA_Dummy + 251);
A(TDTA_Buffer == DTA_Dummy + 300 && TDTA_BufferLen == DTA_Dummy + 301);
A(GID_PICTURE == 0x70696374 && GID_TEXT == 0x74657874 && DTERROR_UNKNOWN_DATATYPE == 2000 && DTERROR_COULDNT_OPEN == 2002);
A(DTA_BaseName == DTA_Dummy + 30 && DTWM_IFF == 0);
A(offsetof(struct dtWrite, dtw_FileHandle) == 8 && offsetof(struct dtWrite, dtw_Mode) == 12 && sizeof(struct dtWrite) == 20);
A(PDTA_SourceMode == DTA_Dummy + 250 && DTA_NominalHoriz == DTA_Dummy + 125 && offsetof(struct BitMapHeader, bmh_PageWidth) == 16 && offsetof(struct BitMapHeader, bmh_PageHeight) == 18);
A(PDTA_BitMap == DTA_Dummy + 202);
